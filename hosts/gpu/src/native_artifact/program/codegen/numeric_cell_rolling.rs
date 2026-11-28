//! Consecutive point updates within one current numeric loan. Source frame
//! recipes prove the successful backedge; every other backedge invalidates.
//! No cached owner/pointer or execution permission survives the native call.
use super::*;
use cranelift_frontend::Variable;
use mech_engine::native_artifact::{
    NativeBufferFrameOperation as Frame, NativeBufferFrames, NativeMatchValue,
    NativeStructuralObservation as Observation, NativeStructuralStep as Path, NativeTailLoop,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Axis {
    Row(i64),
    Column(i64),
    Constant(u64),
}
fn add(a: Axis, c: Axis, subtract: bool) -> Option<Axis> {
    use Axis::*;
    Some(match (a, c, subtract) {
        (Constant(a), Constant(c), false) => Constant(a.checked_add(c)?),
        (Constant(a), Constant(c), true) => Constant(a.checked_sub(c)?),
        (Row(a), Constant(c), false) | (Constant(c), Row(a), false) => {
            Row(a.checked_add(i64::try_from(c).ok()?)?)
        }
        (Column(a), Constant(c), false) | (Constant(c), Column(a), false) => {
            Column(a.checked_add(i64::try_from(c).ok()?)?)
        }
        (Row(a), Constant(c), true) => Row(a.checked_sub(i64::try_from(c).ok()?)?),
        (Column(a), Constant(c), true) => Column(a.checked_sub(i64::try_from(c).ok()?)?),
        (Row(a), Row(c), true) | (Column(a), Column(c), true) => {
            Constant(u64::try_from(a.checked_sub(c)?).ok()?)
        }
        _ => return None,
    })
}
fn classify(
    id: u32,
    row: u32,
    column: u32,
    constants: &BTreeMap<u32, u64>,
    steps: &BTreeMap<u32, &Step>,
    memo: &mut BTreeMap<u32, Axis>,
    depth: usize,
) -> Option<Axis> {
    if depth > 32 {
        return None;
    }
    if let Some(value) = memo.get(&id) {
        return Some(*value);
    }
    let value = if id == row {
        Axis::Row(0)
    } else if id == column {
        Axis::Column(0)
    } else if let Some(value) = constants.get(&id) {
        Axis::Constant(*value)
    } else {
        match *steps.get(&id)? {
            Step::Copy(source) => {
                classify(*source, row, column, constants, steps, memo, depth + 1)?
            }
            Step::Primitive(op, args) => {
                let [a, c] = args.as_slice() else { return None };
                let a = classify(*a, row, column, constants, steps, memo, depth + 1)?;
                let c = classify(*c, row, column, constants, steps, memo, depth + 1)?;
                match op {
                    Op::U64Add => add(a, c, false)?,
                    Op::U64Sub => add(a, c, true)?,
                    Op::U64Mul => match (a, c) {
                        (Axis::Constant(a), Axis::Constant(c)) => Axis::Constant(a.checked_mul(c)?),
                        (a, Axis::Constant(1)) | (Axis::Constant(1), a) => a,
                        (_, Axis::Constant(0)) | (Axis::Constant(0), _) => Axis::Constant(0),
                        _ => return None,
                    },
                    _ => return None,
                }
            }
            _ => return None,
        }
    };
    if matches!(value,Axis::Row(n)|Axis::Column(n) if n.unsigned_abs()>32) {
        return None;
    }
    memo.insert(id, value);
    Some(value)
}
fn projection(
    facts: &NativeProgramFacts<'_>,
    value: u32,
    carrier: u32,
    depth: usize,
) -> Option<Vec<Path>> {
    if depth > 32 {
        return None;
    }
    if value == carrier {
        return Some(Vec::new());
    }
    let [definition] = facts.definition_sites(value).ok()? else {
        return None;
    };
    match facts.plan().blocks()[definition.site()? as usize].action() {
        Action::Scalar(Scalar::Copy { source, .. }) => {
            projection(facts, source.get(), carrier, depth + 1)
        }
        Action::Structural {
            source,
            path,
            observation: Observation::U64 | Observation::CanonicalValue,
            ..
        } => {
            let mut parent = projection(facts, source.get(), carrier, depth + 1)?;
            parent.extend_from_slice(path);
            Some(parent)
        }
        _ => None,
    }
}
fn output(action: &Action) -> Option<NativeMatchValue> {
    match action {
        Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
        | Action::LibraryCall { output, .. } => Some(*output),
        _ => None,
    }
}
fn resolve(
    facts: &NativeProgramFacts<'_>,
    frames: &NativeBufferFrames,
    definitions: &BTreeMap<u32, usize>,
    value: u32,
    path: &[Path],
    depth: usize,
) -> Option<u32> {
    if depth > 32 {
        return None;
    }
    let Some(&site) = definitions.get(&value) else {
        return path.is_empty().then_some(value);
    };
    let block = &facts.plan().blocks()[site];
    if let Action::Scalar(Scalar::Copy { source, .. }) = block.action() {
        return resolve(facts, frames, definitions, source.get(), path, depth + 1);
    }
    match frames.action(site).map(|a| a.operation()) {
        Some(Frame::Copy { source, .. }) => {
            resolve(facts, frames, definitions, source.get(), path, depth + 1)
        }
        _ if path.is_empty() => Some(value),
        Some(Frame::EnumPack {
            payload, ordinal, ..
        }) => {
            let [Path::EnumPayload { ordinal: selected }, rest @ ..] = path else {
                return None;
            };
            if ordinal != selected {
                return None;
            }
            resolve(facts, frames, definitions, payload.get(), rest, depth + 1)
        }
        Some(Frame::Pack { .. }) => {
            let [Path::TupleItem { ordinal }, rest @ ..] = path else {
                return None;
            };
            let Action::LibraryCall { operands, .. } = block.action() else {
                return None;
            };
            resolve(
                facts,
                frames,
                definitions,
                operands.get(*ordinal as usize)?.get(),
                rest,
                depth + 1,
            )
        }
        _ => None,
    }
}
#[derive(Clone, Debug)]
pub(in crate::native_artifact) struct Recipe {
    pub first: usize,
    pub write: usize,
    pub latch: u32,
    pub continuation: usize,
    pub array: usize,
    pub row: u32,
    pub column: u32,
    pub result: u32,
    pub old: Option<u32>,
    pub offsets: BTreeMap<u32, (i64, i64)>,
    pub keys: BTreeSet<(i64, i64)>,
    pub max_column: i64,
    pub max_checked: i64,
    pub new_reads: usize,
}
impl Recipe {
    pub fn derive(
        facts: &NativeProgramFacts<'_>,
        frames: &NativeBufferFrames,
        proof: &Proof,
        tail: &NativeTailLoop,
        prefix: &Prefix,
    ) -> Option<Self> {
        if prefix.exports.len() != 1 || prefix.kinds[&prefix.exports[0]] != Kind::F64 {
            return None;
        }
        let write = prefix.next as usize;
        let Operation::CellWrite {
            array,
            row,
            column,
            incoming,
        } = proof.action(write)?
        else {
            return None;
        };
        if *incoming != prefix.exports[0]
            || row == column
            || proof.arrays[*array].appends
            || !proof.arrays[*array].writable
        {
            return None;
        }
        let last = *prefix.sites.last()?;
        if facts.predecessors(prefix.next).ok()? != [last as u32] {
            return None;
        }
        // No hidden mutation or loan/child boundary can alter cached contents.
        if proof.actions.iter().any(|(site, op)| {
            !matches!(
                op,
                Operation::Frame
                    | Operation::Aggregate
                    | Operation::ArrayCopy
                    | Operation::Length { .. }
                    | Operation::Read { .. }
                    | Operation::CellRead { .. }
            ) && !(*site == write && matches!(op, Operation::CellWrite { .. }))
        }) {
            return None;
        }
        let iteration = facts.plan().blocks()[tail.header() as usize].iteration();
        if proof
            .sites
            .iter()
            .any(|&site| facts.plan().blocks()[site].iteration() != iteration)
        {
            return None;
        }
        let constants = facts
            .plan()
            .constants()
            .iter()
            .filter_map(|v| match v.constant().data() {
                mech_core::ValueData::U64(n) => Some((v.value().get(), *n)),
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        let steps = prefix
            .steps
            .iter()
            .map(|(id, step)| (*id, step))
            .collect::<BTreeMap<_, _>>();
        let mut classes = BTreeMap::new();
        for (&id, &kind) in prefix.inputs.iter().chain(&prefix.kinds) {
            if kind == Kind::U64 {
                classify(id, *row, *column, &constants, &steps, &mut classes, 0)?;
            }
        }
        let mut offsets = BTreeMap::new();
        for (id, step) in &prefix.steps {
            if let Step::Read {
                array: selected,
                row: r,
                column: c,
            } = step
            {
                if selected != array {
                    return None;
                }
                let (Axis::Row(dr), Axis::Column(dc)) = (classes[r], classes[c]) else {
                    return None;
                };
                offsets.insert(*id, (dr, dc));
            }
        }
        let keys = offsets.values().copied().collect::<BTreeSet<_>>();
        let reused = keys
            .iter()
            .filter(|&&(r, c)| (r == 0 && c == -1) || keys.contains(&(r, c + 1)))
            .count();
        if reused < 2 {
            return None;
        }
        let max_column = keys.iter().map(|v| v.1).max()?;
        let max_checked = classes
            .values()
            .filter_map(|c| {
                if let Axis::Column(n) = c {
                    Some(*n)
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);
        let old = offsets
            .iter()
            .find_map(|(&id, &key)| (key == (0, 0)).then_some(id));
        let Action::LibraryCall { output: matrix, .. } = facts.plan().blocks()[write].action()
        else {
            return None;
        };
        let first_read = *prefix.read_sites.first()?;
        let Action::LibraryCall { operands, .. } = facts.plan().blocks()[first_read].action()
        else {
            return None;
        };
        let paths = [
            projection(facts, operands.first()?.get(), tail.carrier().get(), 0)?,
            projection(facts, *row, tail.carrier().get(), 0)?,
            projection(facts, *column, tail.carrier().get(), 0)?,
        ];
        if paths.iter().any(Vec::is_empty) || paths[1] == paths[2] {
            return None;
        }
        // Resolve only this exact linear post-store path, never a different
        // alternative definition of the next carrier at another state edge.
        let mut definitions = BTreeMap::new();
        let mut at = write;
        let mut seen = BTreeSet::new();
        let continuation = loop {
            if seen.len() >= 32 || !seen.insert(at) || !proof.sites.contains(&at) {
                return None;
            }
            let block = &facts.plan().blocks()[at];
            if at != write
                && !matches!(block.action(), Action::Scalar(_))
                && frames.action(at).is_none()
            {
                return None;
            }
            if let Some(value) = output(block.action()) {
                if definitions.insert(value.get(), at).is_some() {
                    return None;
                }
            }
            let next = super::successor(facts, at)?;
            if next == tail.latch() {
                break at;
            }
            at = next as usize;
        };
        let next = tail.next().get();
        if resolve(facts, frames, &definitions, next, &paths[0], 0)? != matrix.get()
            || resolve(facts, frames, &definitions, next, &paths[1], 0)? != *row
        {
            return None;
        }
        let advanced = resolve(facts, frames, &definitions, next, &paths[2], 0)?;
        let site = *definitions.get(&advanced)?;
        let Action::Scalar(Scalar::Primitive {
            op: Op::U64Add,
            operands,
            ..
        }) = facts.plan().blocks()[site].action()
        else {
            return None;
        };
        let [a, c] = operands.as_ref() else {
            return None;
        };
        if !(a.get() == *column && constants.get(&c.get()) == Some(&1)
            || c.get() == *column && constants.get(&a.get()) == Some(&1))
        {
            return None;
        }
        Some(Self {
            first: prefix.first,
            write,
            latch: tail.latch(),
            continuation,
            array: *array,
            row: *row,
            column: *column,
            result: *incoming,
            old,
            offsets,
            new_reads: keys.len() - reused,
            keys,
            max_column,
            max_checked,
        })
    }
    pub fn report(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.first,
            self.write,
            self.continuation,
            self.keys.len(),
            self.new_reads,
        )
    }
}

pub(in crate::native_artifact) struct Cache {
    pub recipe: Recipe,
    valid: Variable,
    pending: Variable,
    max_column: Variable,
    max_checked: Option<Variable>,
    result: Variable,
    values: BTreeMap<(i64, i64), Variable>,
}
impl Cache {
    pub fn new(b: &mut FunctionBuilder<'_>, recipe: Recipe) -> Self {
        let zero = b.ins().iconst(types::I8, 0);
        let word = b.ins().iconst(types::I64, 0);
        let float = b
            .ins()
            .f64const(cranelift_codegen::ir::immediates::Ieee64::with_bits(0));
        let valid = b.declare_var(types::I8);
        b.def_var(valid, zero);
        let pending = b.declare_var(types::I8);
        b.def_var(pending, zero);
        let max_column = b.declare_var(types::I64);
        b.def_var(max_column, word);
        let max_checked = (recipe.max_checked > recipe.max_column).then(|| {
            let v = b.declare_var(types::I64);
            b.def_var(v, word);
            v
        });
        let result = b.declare_var(types::F64);
        b.def_var(result, float);
        let values = recipe
            .keys
            .iter()
            .map(|&key| {
                let v = b.declare_var(types::F64);
                b.def_var(v, float);
                (key, v)
            })
            .collect();
        Self {
            recipe,
            valid,
            pending,
            max_column,
            max_checked,
            result,
            values,
        }
    }
    pub fn invalidate(&self, b: &mut FunctionBuilder<'_>) {
        let zero = b.ins().iconst(types::I8, 0);
        b.def_var(self.valid, zero);
        b.def_var(self.pending, zero);
        // The values are optional derived state. Kill their SSA live ranges,
        // rather than carrying unused data through reset/owner-call edges.
        let word = b.ins().iconst(types::I64, 0);
        b.def_var(self.max_column, word);
        if let Some(at) = self.max_checked {
            b.def_var(at, word);
        }
        let float = b
            .ins()
            .f64const(cranelift_codegen::ir::immediates::Ieee64::with_bits(0));
        b.def_var(self.result, float);
        for &at in self.values.values() {
            b.def_var(at, float);
        }
    }
    pub fn reusable(&self, b: &mut FunctionBuilder<'_>, columns: Value) -> Value {
        let valid = b.use_var(self.valid);
        let last = b.use_var(self.max_column);
        let room = b.ins().icmp(IntCC::UnsignedLessThan, last, columns);
        let mut yes = b.ins().band(valid, room);
        if let Some(at) = self.max_checked {
            let word = b.use_var(at);
            let good = b.ins().icmp_imm(IntCC::NotEqual, word, -1);
            yes = b.ins().band(yes, good);
        }
        yes
    }
    pub fn old_read(&self, b: &mut FunctionBuilder<'_>, key: (i64, i64)) -> Option<Value> {
        if key == (0, -1) {
            Some(b.use_var(self.result))
        } else {
            self.values.get(&(key.0, key.1 + 1)).map(|&v| b.use_var(v))
        }
    }
    pub fn stage(
        &self,
        b: &mut FunctionBuilder<'_>,
        column: Value,
        reads: &BTreeMap<u32, Value>,
        result: Value,
    ) {
        let zero = b.ins().iconst(types::I8, 0);
        b.def_var(self.valid, zero);
        for (&id, &key) in &self.recipe.offsets {
            b.def_var(self.values[&key], reads[&id]);
        }
        b.def_var(self.result, result);
        let max = b.ins().iadd_imm(column, self.recipe.max_column);
        b.def_var(self.max_column, max);
        if let Some(at) = self.max_checked {
            let max = b.ins().iadd_imm(column, self.recipe.max_checked);
            b.def_var(at, max);
        }
        let one = b.ins().iconst(types::I8, 1);
        b.def_var(self.pending, one);
    }
    pub fn finish_write(&self, b: &mut FunctionBuilder<'_>, site: usize) {
        if site == self.recipe.write {
            let yes = b.use_var(self.pending);
            b.def_var(self.valid, yes);
        }
    }
    pub fn control(
        &self,
        b: &mut FunctionBuilder<'_>,
        facts: &NativeProgramFacts<'_>,
        site: usize,
    ) {
        if super::successor(facts, site) == Some(self.recipe.latch)
            && site != self.recipe.continuation
        {
            self.invalidate(b);
        }
    }
}
#[cfg(all(test, feature = "canonical-native-program-runtime"))]
#[path = "numeric_cell_rolling_tests.rs"]
mod tests;
