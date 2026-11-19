//! Current read bounds recorded only after numeric source admission. These
//! hints are not serialized or restored from public artifact metadata.
use super::*;

pub(in crate::native_artifact) struct BoundedRead {
    load: Inst,
    array: usize,
}

impl BoundedRead {
    /// Call immediately after emitting a scalar byte read inside its original
    /// logical index check or the existing current-window proof. No hint is
    /// valid for a physical load that merely happens to be readable.
    pub(in crate::native_artifact::program::codegen) fn new(
        f: &Function,
        value: Value,
        array: usize,
    ) -> Option<Self> {
        let ValueDef::Result(load, 0) = f.dfg.value_def(value) else {
            return None;
        };
        matches!(
            f.dfg.insts[load],
            Data::Load {
                opcode: Op::Uload8,
                ..
            }
        )
        .then_some(Self { load, array })
    }

    pub(super) fn keys(
        &self,
        f: &Function,
        cfg: &ControlFlowGraph,
        value: Value,
        views: &BTreeMap<usize, (Value, Value)>,
    ) -> Option<(Address, Address)> {
        if f.dfg.value_def(value) != ValueDef::Result(self.load, 0)
            || f.layout.inst_block(self.load).is_none()
        {
            return None;
        }
        let &(pointer, length) = views.get(&self.array)?;
        let Data::Load {
            opcode: Op::Uload8,
            arg,
            offset,
            ..
        } = f.dfg.insts[self.load]
        else {
            return None;
        };
        let mut symbols = Symbols::default();
        let address = symbols.existing(arg);
        let mut index = symbols.address(f, cfg, address, i32::from(offset))?;
        // Capture no temporary ordinal or length SSA names: cleanup may
        // invalidate them. This surviving source load still addresses its
        // exact fixed current vector; subtract its current packet pointer.
        if index.words.remove(&current(f, cfg, pointer)) != Some(1) {
            return None;
        }
        let length = symbols.existing(length);
        Some((index, symbols.address(f, cfg, length, 0)?))
    }
}

impl Walk<'_> {
    pub(super) fn admitted(&mut self, values: &Values, condition: Value, byte: usize) -> bool {
        let Some((index, length)) = self.bytes[byte].bounds.clone() else {
            return false;
        };
        let Some(compare) = comparison(self.f, condition) else {
            return false;
        };
        let args = match self.f.dfg.insts[compare] {
            Data::IntCompare {
                cond: IntCC::UnsignedLessThan,
                args,
                ..
            } => args,
            Data::IntCompare {
                cond: IntCC::UnsignedGreaterThan,
                args: [length, index],
                ..
            } => [index, length],
            _ => return false,
        };
        let Some(actual) = args
            .map(|v| {
                self.value(values, v)
                    .and_then(|s| self.symbols.address(self.f, self.cfg, s, 0))
            })
            .into_iter()
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        actual == [index, length]
    }
}
