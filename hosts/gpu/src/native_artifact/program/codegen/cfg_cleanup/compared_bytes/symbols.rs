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
}
#[derive(Default)]
pub(super) struct Symbols {
    nodes: Vec<Node>,
}
#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct Address {
    constant: u64,
    pub words: BTreeMap<Value, u64>,
}
impl Address {
    fn add(mut self, other: Self) -> Self {
        self.constant = self.constant.wrapping_add(other.constant);
        for (value, times) in other.words {
            let n = self.words.entry(value).or_default();
            *n = n.wrapping_add(times);
        }
        self.words.retain(|_, times| *times != 0);
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
        let mut budget = 256;
        let mut key = self.key(f, cfg, symbol, 0, &mut budget)?;
        key.constant = key.constant.wrapping_add(offset as i64 as u64);
        Some(key)
    }
    fn key(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        symbol: Symbol,
        depth: usize,
        budget: &mut usize,
    ) -> Option<Address> {
        if depth > 32 || *budget == 0 {
            return None;
        }
        *budget -= 1;
        match &self.nodes[symbol] {
            Node::Choice(..) => None,
            Node::Existing(value) => self.original_key(f, cfg, *value, depth + 1, budget),
            Node::Copy(inst, args) => self.instruction_key(f, cfg, *inst, args, depth + 1, budget),
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
                        words: BTreeMap::new(),
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
            constant: 0,
            words: BTreeMap::from([(value, 1)]),
        })
    }
    fn instruction_key(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        inst: Inst,
        args: &[Symbol],
        depth: usize,
        budget: &mut usize,
    ) -> Option<Address> {
        match f.dfg.insts[inst] {
            Data::UnaryImm {
                opcode: Op::Iconst,
                imm,
            } => Some(Address {
                constant: imm.bits() as u64,
                words: BTreeMap::new(),
            }),
            Data::Binary {
                opcode: Op::Iadd, ..
            } => Some(self.key(f, cfg, args[0], depth + 1, budget)?.add(self.key(
                f,
                cfg,
                args[1],
                depth + 1,
                budget,
            )?)),
            Data::BinaryImm64 {
                opcode: Op::IaddImm,
                imm,
                ..
            } => {
                let mut key = self.key(f, cfg, args[0], depth + 1, budget)?;
                key.constant = key.constant.wrapping_add(imm.bits() as u64);
                Some(key)
            }
            _ => None,
        }
    }
    pub fn emit(&self, cursor: &mut FuncCursor<'_>, symbols: &[Symbol]) -> Vec<Value> {
        fn value(
            nodes: &[Node],
            cache: &mut BTreeMap<Symbol, Value>,
            c: &mut FuncCursor<'_>,
            at: Symbol,
        ) -> Value {
            if let Some(&result) = cache.get(&at) {
                return result;
            }
            let result = match &nodes[at] {
                Node::Existing(value) => *value,
                Node::Copy(inst, args) => {
                    let args = args
                        .iter()
                        .map(|&arg| value(nodes, cache, c, arg))
                        .collect::<Vec<_>>();
                    let copy = c.func.dfg.clone_inst(*inst);
                    c.func.dfg.inst_args_mut(copy).copy_from_slice(&args);
                    c.insert_inst(copy);
                    c.func.dfg.first_result(copy)
                }
                Node::Choice(condition, yes, no) => {
                    let condition = value(nodes, cache, c, *condition);
                    let yes = value(nodes, cache, c, *yes);
                    let no = value(nodes, cache, c, *no);
                    c.ins().select(condition, yes, no)
                }
            };
            cache.insert(at, result);
            result
        }
        let mut cache = BTreeMap::new();
        symbols
            .iter()
            .map(|&symbol| value(&self.nodes, &mut cache, cursor, symbol))
            .collect()
    }
}
