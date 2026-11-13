//! Prove the selected path is a single wrapping increment and a unit latch.
use super::super::composed_reads::packet_word;
use super::super::word_ranges::constant_offset;
use super::*;
use std::collections::BTreeSet;

pub(super) struct Arm<'a> {
    pub view: &'a View,
    pub key_mask: u32,
    pub key_shift: u32,
}

struct Path<'a> {
    f: &'a Function,
    parameters: BTreeMap<Value, Value>,
    seen: BTreeSet<Block>,
    checks: Vec<(Value, Value)>,
    memory: Vec<Inst>,
}

impl Path<'_> {
    fn resolve(&self, value: Value) -> Value {
        let value = self.f.dfg.resolve_aliases(value);
        self.parameters.get(&value).copied().unwrap_or(value)
    }

    fn follow(&mut self, ctx: &Context, mut edge: BlockCall, header: Block) -> Option<Value> {
        let f = self.f;
        loop {
            let block = edge.block(&f.dfg.value_lists);
            let mut args = edge.args(&f.dfg.value_lists);
            if block == header {
                return Some(self.resolve(args.next()?.as_value()?));
            }
            // The latch must return directly to this iteration boundary.
            // Reentering a preheader could redefine a supposedly fixed word.
            if !ctx.domtree.block_dominates(header, block) || !self.seen.insert(block) {
                return None;
            }
            for &parameter in f.dfg.block_params(block) {
                let arg = self.resolve(args.next()?.as_value()?);
                self.parameters.insert(parameter, arg);
            }
            let last = f.layout.last_inst(block)?;
            for inst in f.layout.block_insts(block).take_while(|&inst| inst != last) {
                let opcode = f.dfg.insts[inst].opcode();
                if matches!(opcode, Op::Uload32 | Op::Store) {
                    if self.memory.len() == 2 {
                        return None;
                    }
                    self.memory.push(inst);
                } else if !scalar(opcode) {
                    return None;
                }
            }
            edge = match f.dfg.insts[last] {
                Data::Jump { destination, .. } => destination,
                Data::Brif { .. } => {
                    let (sense, left, Operand::Value(right), arms) = predicate(f, last)? else {
                        return None;
                    };
                    let selected = match sense {
                        IntCC::UnsignedLessThan => 0,
                        IntCC::UnsignedGreaterThanOrEqual => 1,
                        _ => return None,
                    };
                    self.checks.push((self.resolve(left), self.resolve(right)));
                    arms[selected]
                }
                _ => return None,
            };
        }
    }

    fn index(&self, address: Value) -> Option<(Value, Value)> {
        let f = self.f;
        let Data::Binary {
            opcode: Op::Iadd,
            args: [a, b],
        } = definition(f, self.resolve(address))?
        else {
            return None;
        };
        let scaled = |value| {
            let Data::BinaryImm64 { opcode, arg, imm } = definition(f, self.resolve(value))? else {
                return None;
            };
            if !matches!((opcode, imm.bits()), (Op::ImulImm, 4) | (Op::IshlImm, 2)) {
                return None;
            }
            let (base, bias) = constant_offset(f, self.resolve(arg));
            (bias == 0).then(|| self.resolve(base))
        };
        scaled(b)
            .map(|key| (self.resolve(a), key))
            .or_else(|| scaled(a).map(|key| (self.resolve(b), key)))
    }

    fn unit_latch(&self, next: Value, cursor: Value, length: Value) -> bool {
        let f = self.f;
        if let Some(Data::Binary {
            opcode: Op::Iadd,
            args,
        }) = definition(f, next)
        {
            let [left, right] = args.map(|arg| self.resolve(arg));
            if (left == cursor && right == length) || (right == cursor && left == length) {
                return true;
            }
        }
        let (base, bias) = constant_offset(f, next);
        self.resolve(base) == cursor && bias == 1
    }
}

pub(super) fn prove<'a>(
    ctx: &Context,
    edge: BlockCall,
    header: Block,
    cursor: Value,
    length: Value,
    packed: Value,
    loads: &BTreeMap<Value, (Inst, usize)>,
    bins: &'a [View],
) -> Option<Arm<'a>> {
    let f = &ctx.func;
    let mut path = Path {
        f,
        parameters: BTreeMap::new(),
        seen: BTreeSet::new(),
        checks: Vec::new(),
        memory: Vec::new(),
    };
    let next = path.follow(ctx, edge, header)?;
    if !path.unit_latch(next, cursor, length) {
        return None;
    }
    let [load, store] = *path.memory.as_slice() else {
        return None;
    };
    let Data::Load {
        opcode: Op::Uload32,
        arg,
        offset,
        flags,
    } = f.dfg.insts[load]
    else {
        return None;
    };
    let (pointer, key) = path.index(arg)?;
    let word = |value| packet_word(f, &ctx.cfg, loads, value);
    let pointer_word = word(pointer)?;
    let view = bins
        .iter()
        .find(|v| v.width == 4 && v.pointer_word == pointer_word)?;
    if i32::from(offset) != 0
        || flags != MemFlags::new()
        || f.dfg.value_type(f.dfg.first_result(load)) != types::I64
        || path.checks.iter().any(|&(index, length)| {
            let (base, bias) = constant_offset(f, index);
            path.resolve(base) != key || bias != 0 || word(length) != Some(view.length_word)
        })
    {
        return None;
    }
    let Data::Store {
        opcode: Op::Store,
        args: [result, address],
        offset,
        flags,
    } = f.dfg.insts[store]
    else {
        return None;
    };
    let (store_pointer, store_key) = path.index(address)?;
    if i32::from(offset) != 0
        || flags != MemFlags::new()
        || word(store_pointer) != Some(pointer_word)
        || store_key != key
        || f.dfg.value_type(result) != types::I32
    {
        return None;
    }
    let Data::Unary {
        opcode: Op::Ireduce,
        arg,
    } = definition(f, path.resolve(result))?
    else {
        return None;
    };
    let (sum, mask) = with_constant(f, path.resolve(arg), Op::Band, Op::BandImm, true)?;
    let (count, step) = with_constant(f, path.resolve(sum), Op::Iadd, Op::IaddImm, true)?;
    if mask != u64::from(u32::MAX) || step != 1 || path.resolve(count) != f.dfg.first_result(load) {
        return None;
    }
    let (shifted, key_mask) = with_constant(f, key, Op::Band, Op::BandImm, true)?;
    let (source, key_shift) =
        with_constant(f, path.resolve(shifted), Op::Ushr, Op::UshrImm, false)?;
    if path.resolve(source) != packed || key_shift >= 32 {
        return None;
    }
    Some(Arm {
        view,
        key_mask: u32::try_from(key_mask).ok()?,
        key_shift: u32::try_from(key_shift).ok()?,
    })
}
