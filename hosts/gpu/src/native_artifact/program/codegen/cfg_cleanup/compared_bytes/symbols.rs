use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    flowgraph::ControlFlowGraph,
    ir::{
        Function, Inst, InstBuilder, InstructionData as Data, Opcode as Op, Value, ValueDef, types,
    },
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Symbol = usize;
#[derive(Clone, PartialEq, Eq)]
enum Node {
    Existing(Value),
    Copy(Inst, Vec<Symbol>),
    Choice(Symbol, Symbol, Symbol),
    WordByte(Value, Symbol),
}
#[derive(Default)]
pub(super) struct Symbols {
    nodes: Vec<Node>,
}
#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct Address {
    constant: u64,
    pub words: BTreeMap<Value, u64>,
    byte_offset: u64,
}
impl Address {
    fn add(mut self, other: Self) -> Self {
        self.constant = self.constant.wrapping_add(other.constant);
        self.byte_offset = self.byte_offset.wrapping_add(other.byte_offset);
        for (value, times) in other.words {
            let n = self.words.entry(value).or_default();
            *n = n.wrapping_add(times);
        }
        self.words.retain(|_, times| *times != 0);
        self
    }

    pub(super) fn plus_byte_offset(mut self) -> Self {
        self.byte_offset = self.byte_offset.wrapping_add(1);
        self
    }
}

pub(super) fn current(f: &Function, cfg: &ControlFlowGraph, value: Value) -> Value {
    // First peel exact forwarding joins. If the eventual value is a dynamic
    // phi, retain that phi rather than each later transport parameter's name.
    let mut original = f.dfg.resolve_aliases(value);
    let mut transport = BTreeSet::new();
    while transport.len() < 128 && transport.insert(original) {
        let ValueDef::Param(block, slot) = f.dfg.value_def(original) else {
            break;
        };
        if Some(block) == f.layout.entry_block() {
            break;
        }
        let mut arguments = BTreeSet::new();
        for pred in cfg.pred_iter(block) {
            for call in f.dfg.insts[pred.inst]
                .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
            {
                if call.block(&f.dfg.value_lists) == block {
                    let Some(value) = call
                        .args(&f.dfg.value_lists)
                        .nth(slot)
                        .and_then(|arg| arg.as_value())
                    else {
                        return original;
                    };
                    let value = f.dfg.resolve_aliases(value);
                    if value != original {
                        arguments.insert(value);
                    }
                }
            }
        }
        if arguments.len() != 1 {
            break;
        }
        let next = *arguments.first().expect("one forwarding value");
        if transport.contains(&next) {
            break;
        }
        original = next;
    }
    let mut pending = vec![original];
    let mut seen = BTreeSet::new();
    let mut leaves = BTreeSet::new();
    while let Some(value) = pending.pop() {
        let value = f.dfg.resolve_aliases(value);
        if !seen.insert(value) {
            continue;
        }
        if seen.len() > 128 {
            return original;
        }
        if let ValueDef::Param(block, slot) = f.dfg.value_def(value)
            && Some(block) != f.layout.entry_block()
        {
            let mut incoming = false;
            for pred in cfg.pred_iter(block) {
                for call in f.dfg.insts[pred.inst]
                    .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                {
                    if call.block(&f.dfg.value_lists) == block {
                        let Some(arg) = call
                            .args(&f.dfg.value_lists)
                            .nth(slot)
                            .and_then(|arg| arg.as_value())
                        else {
                            return original;
                        };
                        pending.push(arg);
                        incoming = true;
                    }
                }
            }
            if !incoming {
                return original;
            }
        } else {
            leaves.insert(value);
            if leaves.len() > 1 {
                return original;
            }
        }
    }
    leaves.first().copied().unwrap_or(original)
}

impl Symbols {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    fn intern(&mut self, node: Node) -> Symbol {
        if let Some(at) = self.nodes.iter().position(|old| *old == node) {
            return at;
        }
        let at = self.nodes.len();
        self.nodes.push(node);
        at
    }
    pub fn existing(&mut self, value: Value) -> Symbol {
        self.intern(Node::Existing(value))
    }
    pub fn copy(&mut self, inst: Inst, args: Vec<Symbol>) -> Symbol {
        self.intern(Node::Copy(inst, args))
    }
    pub fn choice(&mut self, condition: Symbol, yes: Symbol, no: Symbol) -> Symbol {
        if yes == no {
            yes
        } else {
            self.intern(Node::Choice(condition, yes, no))
        }
    }
    pub(super) fn word_byte(&mut self, word: Value, byte_offset: Symbol) -> Symbol {
        self.intern(Node::WordByte(word, byte_offset))
    }

    pub(super) fn byte_word(&self, symbol: Symbol) -> Option<Value> {
        match self.nodes.get(symbol)? {
            Node::WordByte(word, _) => Some(*word),
            _ => None,
        }
    }

    pub(super) fn depends_on_word_byte(&self, symbol: Symbol) -> bool {
        let mut pending = vec![symbol];
        let mut visited = BTreeSet::new();
        while let Some(symbol) = pending.pop() {
            if !visited.insert(symbol) {
                continue;
            }
            match &self.nodes[symbol] {
                Node::WordByte(..) => return true,
                Node::Copy(_, args) => pending.extend(args),
                Node::Choice(condition, yes, no) => pending.extend([*condition, *yes, *no]),
                Node::Existing(_) => {}
            }
        }
        false
    }

    fn constant_word(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        symbol: Symbol,
        depth: usize,
    ) -> Option<u64> {
        if depth > 8 {
            return None;
        }
        let inst = match &self.nodes[symbol] {
            Node::Existing(value) => {
                return super::super::word_ranges::literal(f, current(f, cfg, *value));
            }
            Node::Copy(inst, _) => *inst,
            _ => return None,
        };
        match f.dfg.insts[inst] {
            Data::UnaryImm {
                opcode: Op::Iconst,
                imm,
            } => Some(imm.bits() as u64),
            Data::BinaryImm64 {
                opcode: Op::BandImm,
                imm,
                ..
            } => {
                let Node::Copy(_, args) = &self.nodes[symbol] else {
                    return None;
                };
                Some(self.constant_word(f, cfg, args[0], depth + 1)? & imm.bits() as u64)
            }
            _ => None,
        }
    }

    /// A nonzero I64 XOR makes this exact byte offset fall within its word.
    pub(super) fn first_bad_byte(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        symbol: Symbol,
        difference: Value,
    ) -> bool {
        let Node::Copy(shift, args) = &self.nodes[symbol] else {
            return false;
        };
        if f.dfg.value_type(f.dfg.first_result(*shift)) != types::I64 {
            return false;
        }
        let count = match f.dfg.insts[*shift] {
            Data::Binary {
                opcode: Op::Ushr, ..
            } if self.constant_word(f, cfg, args[1], 0) == Some(3) => args[0],
            Data::BinaryImm64 {
                opcode: Op::UshrImm,
                imm,
                ..
            } if imm.bits() == 3 => args[0],
            _ => return false,
        };
        let Node::Copy(ctz, args) = &self.nodes[count] else {
            return false;
        };
        f.dfg.insts[*ctz].opcode() == Op::Ctz
            && matches!(self.nodes[args[0]], Node::Existing(value) if current(f, cfg, value) == difference)
    }
    pub fn zero(&self, f: &Function, symbol: Symbol) -> bool {
        match &self.nodes[symbol] {
            Node::Existing(value) => {
                matches!(f.dfg.value_def(f.dfg.resolve_aliases(*value)), ValueDef::Result(inst, 0)
                if matches!(f.dfg.insts[inst], Data::UnaryImm {opcode: Op::Iconst, imm} if imm.bits()==0))
            }
            Node::Copy(inst, _) => {
                matches!(f.dfg.insts[*inst], Data::UnaryImm {opcode: Op::Iconst, imm} if imm.bits()==0)
            }
            _ => false,
        }
    }
    pub fn address(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        symbol: Symbol,
        offset: i32,
    ) -> Option<Address> {
        self.address_with_offset(f, cfg, symbol, offset, None)
    }

    pub(super) fn address_with_offset(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        symbol: Symbol,
        offset: i32,
        byte_offset: Option<Symbol>,
    ) -> Option<Address> {
        let mut budget = 256;
        let mut key = self.key(f, cfg, symbol, byte_offset, 0, &mut budget)?;
        key.constant = key.constant.wrapping_add(offset as i64 as u64);
        Some(key)
    }
    fn key(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        symbol: Symbol,
        byte_offset: Option<Symbol>,
        depth: usize,
        budget: &mut usize,
    ) -> Option<Address> {
        if depth > 32 || *budget == 0 {
            return None;
        }
        *budget -= 1;
        if Some(symbol) == byte_offset {
            return Some(Address::default().plus_byte_offset());
        }
        match &self.nodes[symbol] {
            Node::Choice(..) | Node::WordByte(..) => None,
            Node::Existing(value) => self.original_key(f, cfg, *value, depth + 1, budget),
            Node::Copy(inst, args) => {
                self.instruction_key(f, cfg, *inst, args, byte_offset, depth + 1, budget)
            }
        }
    }
    fn original_key(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        value: Value,
        depth: usize,
        budget: &mut usize,
    ) -> Option<Address> {
        if depth > 32 || *budget == 0 {
            return None;
        }
        *budget -= 1;
        let value = current(f, cfg, value);
        if f.dfg.value_type(value) != types::I64 {
            return None;
        }
        if let ValueDef::Result(inst, 0) = f.dfg.value_def(value) {
            let data = f.dfg.insts[inst];
            match data {
                Data::UnaryImm {
                    opcode: Op::Iconst,
                    imm,
                } => {
                    return Some(Address {
                        constant: imm.bits() as u64,
                        ..Address::default()
                    });
                }
                Data::Binary {
                    opcode: Op::Iadd,
                    args: [a, b],
                } => {
                    return Some(
                        self.original_key(f, cfg, a, depth + 1, budget)?
                            .add(self.original_key(f, cfg, b, depth + 1, budget)?),
                    );
                }
                Data::BinaryImm64 {
                    opcode: Op::IaddImm,
                    arg,
                    imm,
                } => {
                    let mut key = self.original_key(f, cfg, arg, depth + 1, budget)?;
                    key.constant = key.constant.wrapping_add(imm.bits() as u64);
                    return Some(key);
                }
                _ => {}
            }
        }
        Some(Address {
            words: BTreeMap::from([(value, 1)]),
            ..Address::default()
        })
    }
    fn instruction_key(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        inst: Inst,
        args: &[Symbol],
        byte_offset: Option<Symbol>,
        depth: usize,
        budget: &mut usize,
    ) -> Option<Address> {
        match f.dfg.insts[inst] {
            Data::UnaryImm {
                opcode: Op::Iconst,
                imm,
            } => Some(Address {
                constant: imm.bits() as u64,
                ..Address::default()
            }),
            Data::Binary {
                opcode: Op::Iadd, ..
            } => Some(
                self.key(f, cfg, args[0], byte_offset, depth + 1, budget)?
                    .add(self.key(f, cfg, args[1], byte_offset, depth + 1, budget)?),
            ),
            Data::BinaryImm64 {
                opcode: Op::IaddImm,
                imm,
                ..
            } => {
                let mut key = self.key(f, cfg, args[0], byte_offset, depth + 1, budget)?;
                key.constant = key.constant.wrapping_add(imm.bits() as u64);
                Some(key)
            }
            _ => None,
        }
    }
    /// Evaluate only exact booleans and comparisons known from dominating
    /// edges. Copies use the values transported along the replayed edge.
    pub(super) fn truth(
        &mut self,
        f: &Function,
        cfg: &ControlFlowGraph,
        symbol: Symbol,
        facts: &[super::Fact],
        byte_offset: Option<Symbol>,
        depth: usize,
    ) -> Option<bool> {
        use cranelift_codegen::ir::condcodes::IntCC;
        if depth > 24 || self.nodes.len() >= 512 {
            return None;
        }
        let (inst, args) = match self.nodes.get(symbol)?.clone() {
            Node::Existing(value) => {
                let ValueDef::Result(inst, 0) = f.dfg.value_def(current(f, cfg, value)) else {
                    return None;
                };
                let args = f
                    .dfg
                    .inst_args(inst)
                    .iter()
                    .map(|&arg| self.existing(arg))
                    .collect();
                (inst, args)
            }
            Node::Copy(inst, args) => (inst, args),
            Node::Choice(..) | Node::WordByte(..) => return None,
        };
        let mut next = |value| self.truth(f, cfg, value, facts, byte_offset, depth + 1);
        match f.dfg.insts[inst] {
            Data::UnaryImm {
                opcode: Op::Iconst,
                imm,
            } => match imm.bits() {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            },
            Data::Unary {
                opcode: Op::Uextend | Op::Ireduce,
                ..
            } => next(args[0]),
            Data::BinaryImm64 {
                opcode: Op::BandImm,
                imm,
                ..
            } if imm.bits() == 1 => next(args[0]),
            Data::Binary {
                opcode: Op::Band, ..
            } => match (next(args[0]), next(args[1])) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            },
            Data::IntCompare { cond, .. } => {
                let left = self.address_with_offset(f, cfg, args[0], 0, byte_offset)?;
                let right = self.address_with_offset(f, cfg, args[1], 0, byte_offset)?;
                if left == right {
                    return match cond {
                        IntCC::Equal
                        | IntCC::UnsignedLessThanOrEqual
                        | IntCC::UnsignedGreaterThanOrEqual => Some(true),
                        IntCC::NotEqual | IntCC::UnsignedLessThan | IntCC::UnsignedGreaterThan => {
                            Some(false)
                        }
                        _ => None,
                    };
                }
                facts
                    .iter()
                    .find_map(|fact| fact.relates(cond, &left, &right))
            }
            _ => None,
        }
    }

    pub fn emit(
        &self,
        cursor: &mut FuncCursor<'_>,
        symbols: &[Symbol],
        reverse_words: bool,
    ) -> Vec<Value> {
        fn value(
            nodes: &[Node],
            cache: &mut BTreeMap<Symbol, Value>,
            c: &mut FuncCursor<'_>,
            at: Symbol,
            reverse_words: bool,
        ) -> Value {
            if let Some(&result) = cache.get(&at) {
                return result;
            }
            let result = match &nodes[at] {
                Node::Existing(value) => *value,
                Node::Copy(inst, args) => {
                    let args = args
                        .iter()
                        .map(|&arg| value(nodes, cache, c, arg, reverse_words))
                        .collect::<Vec<_>>();
                    let copy = c.func.dfg.clone_inst(*inst);
                    c.func.dfg.inst_args_mut(copy).copy_from_slice(&args);
                    c.insert_inst(copy);
                    c.func.dfg.first_result(copy)
                }
                Node::Choice(condition, yes, no) => {
                    let condition = value(nodes, cache, c, *condition, reverse_words);
                    let yes = value(nodes, cache, c, *yes, reverse_words);
                    let no = value(nodes, cache, c, *no, reverse_words);
                    c.ins().select(condition, yes, no)
                }
                Node::WordByte(word, _) if reverse_words => c.ins().bswap(*word),
                Node::WordByte(word, byte_offset) => {
                    let byte_offset = value(nodes, cache, c, *byte_offset, reverse_words);
                    let shift = c.ins().ishl_imm(byte_offset, 3);
                    let shifted = c.ins().ushr(*word, shift);
                    c.ins().band_imm(shifted, 255)
                }
            };
            cache.insert(at, result);
            result
        }
        let mut cache = BTreeMap::new();
        symbols
            .iter()
            .map(|&symbol| value(&self.nodes, &mut cache, cursor, symbol, reverse_words))
            .collect()
    }
}
