//! Native collection builders and fresh selected range cursors. All loop
//! bodies remain in the shared source CFG; certified uniform regions alone
//! use their existing same-Plan proof to construct an exact initialized prefix.
use super::{
    arrays::{self, Arrays},
    constants::Initial,
    layout::Layout,
    packed,
    word_collections::Kernel,
};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::PreparedNativeProgram;
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, guard},
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, Value, types};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::Module;
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as P, NativeCollectionResult as ResultKind,
    NativeCollectionSource as Source, NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeProgramArtifactPlan as Plan,
};
use mech_engine::{ComprehensionKind, MatrixBlockRows};
use std::collections::BTreeMap;
const INVALID: u64 = 0x19403;
#[derive(Clone)]
struct Cursor {
    start: Variable,
    count: Variable,
    index: Variable,
}
pub(super) struct Collections {
    builders: Vec<Vec<Variable>>,
    cursors: Vec<Option<Cursor>>,
    rows_set: Vec<Variable>,
}
fn variable(b: &mut FunctionBuilder<'_>, zero: Value) -> Variable {
    let v = b.declare_var(types::I64);
    b.def_var(v, zero);
    v
}
impl Collections {
    pub fn new(
        b: &mut FunctionBuilder<'_>,
        plan: &Plan,
        zero: Value,
    ) -> Result<Self, NativeCompileError> {
        for definition in plan.collections() {
            if definition.generators().is_empty()
                || (definition.kind() == ComprehensionKind::MatrixPreserveShape
                    && definition.generators().len() != 1)
                || !matches!(definition.result(), ResultKind::Materialized)
                || !matches!(
                    definition.kind(),
                    ComprehensionKind::Matrix
                        | ComprehensionKind::MatrixPreserveShape
                        | ComprehensionKind::MatrixHorizontalBlocks { .. }
                )
            {
                return Err(backend(
                    "compact collections require one actual numeric matrix generator",
                ));
            }
        }
        let builders = plan
            .collections()
            .iter()
            .map(|_| (0..6).map(|_| variable(b, zero)).collect())
            .collect();
        let cursors = plan
            .iterations()
            .iter()
            .map(|i| {
                if matches!(i, Loop::Collection(_)) {
                    Some(Cursor {
                        start: variable(b, zero),
                        count: variable(b, zero),
                        index: variable(b, zero),
                    })
                } else {
                    None
                }
            })
            .collect();
        let rows_set = plan
            .collections()
            .iter()
            .map(|_| variable(b, zero))
            .collect();
        Ok(Self {
            builders,
            cursors,
            rows_set,
        })
    }
    fn cursor(&self, id: u32) -> Result<&Cursor, NativeCompileError> {
        self.cursors[id as usize]
            .as_ref()
            .ok_or_else(|| backend("compact collection cursor is not a source generator"))
    }
    fn view(&self, b: &mut FunctionBuilder<'_>, id: u32) -> Vec<Value> {
        self.builders[id as usize]
            .iter()
            .map(|&v| b.use_var(v))
            .collect()
    }
    fn set(&self, b: &mut FunctionBuilder<'_>, id: u32, view: &[Value]) {
        for (&v, &word) in self.builders[id as usize].iter().zip(view) {
            b.def_var(v, word)
        }
    }
    fn replace(
        &self,
        b: &mut FunctionBuilder<'_>,
        arrays: &Arrays,
        id: u32,
        view: &[Value],
        error: Block,
    ) {
        let previous = self.view(b, id)[5];
        arrays.retain(b, view[5]);
        self.set(b, id, view);
        arrays.release(b, previous, error);
    }
    pub fn after(
        &self,
        b: &mut FunctionBuilder<'_>,
        arrays: &Arrays,
        action: &Action,
        error: Block,
    ) {
        if let Action::CollectionFinish { collection } = action {
            let owner = self.view(b, *collection)[5];
            let zero = b.ins().iconst(types::I64, 0);
            b.def_var(self.builders[*collection as usize][5], zero);
            arrays.release(b, owner, error);
        }
    }
    pub fn action(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        layout: &Layout,
        arrays: &Arrays,
        plan: &Plan,
        action: &Action,
        error: Block,
    ) -> Result<bool, NativeCompileError> {
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        match action {
            Action::CollectionBegin { collection } => {
                let previous = self.view(b, *collection)[5];
                let (rows, known) = match plan.collections()[*collection as usize].kind() {
                    ComprehensionKind::MatrixHorizontalBlocks {
                        rows: MatrixBlockRows::Fixed(n),
                    } => (b.ins().iconst(types::I64, n as i64), one),
                    ComprehensionKind::MatrixHorizontalBlocks {
                        rows: MatrixBlockRows::InferOrZero,
                    } => (zero, zero),
                    ComprehensionKind::MatrixHorizontalBlocks {
                        rows: MatrixBlockRows::InputMatrixElements { input },
                    } => {
                        let source = plan
                            .inputs()
                            .get(input as usize)
                            .ok_or_else(|| backend("native block rows input absent"))?
                            .value();
                        layout.ty(source.get()).element()?;
                        (words.read(b, layout.slot(source.get()) + 1), one)
                    }
                    _ => (one, one),
                };
                b.def_var(self.rows_set[*collection as usize], known);
                self.set(b, *collection, &[zero, zero, zero, rows, zero, zero]);
                arrays.release(b, previous, error);
            }
            Action::PrepareU64Range { iteration } => {
                let Loop::Collection(generator) = &plan.iterations()[*iteration as usize] else {
                    return Err(backend("compact range lacks checked source collection"));
                };
                let Source::U64Range(source) = generator.source() else {
                    return Err(backend("compact range is not a checked selected range"));
                };
                let [start, end] = source
                    .operands()
                    .map(|v| words.read(b, layout.slot(v.get())));
                let valid = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, end, start);
                guard(b, valid, error, INVALID);
                let mut count = b.ins().isub(end, start);
                match source.template().primitive() {
                    P::RangeInclusive => {
                        let (sum, overflow) = b.ins().uadd_overflow(count, one);
                        let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
                        guard(b, valid, error, INVALID);
                        count = sum;
                    }
                    P::RangeExclusive => {}
                    _ => {
                        return Err(backend(
                            "compact range primitive is not inclusive/exclusive",
                        ));
                    }
                }
                let positive = b.ins().icmp_imm(IntCC::NotEqual, count, 0);
                guard(b, positive, error, INVALID);
                // The source range's original representation bound is retained
                // even though this checked cursor does not allocate its Matrix.
                let maximum = isize::MAX as usize
                    / std::mem::size_of::<mech_core::snapshot::ValueDataDraft>()
                    - 2;
                let fits = b
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThanOrEqual, count, maximum as i64);
                guard(b, fits, error, INVALID);
                let cursor = self.cursor(*iteration)?;
                b.def_var(cursor.start, start);
                b.def_var(cursor.count, count);
            }
            Action::Initialize { iteration } => {
                let Loop::Collection(generator) = &plan.iterations()[*iteration as usize] else {
                    return Ok(false);
                };
                let cursor = self.cursor(*iteration)?;
                b.def_var(cursor.index, zero);
                if let Source::Value(source) = generator.source() {
                    let source = arrays::read(words, b, layout, *source);
                    b.def_var(cursor.count, source[1]);
                }
            }
            Action::Latch { iteration } => {
                if !matches!(plan.iterations()[*iteration as usize], Loop::Collection(_)) {
                    return Ok(false);
                }
                let cursor = self.cursor(*iteration)?;
                let old = b.use_var(cursor.index);
                let next = b.ins().iadd_imm(old, 1);
                b.def_var(cursor.index, next);
            }
            Action::CollectionElement { iteration } => {
                let Loop::Collection(generator) = &plan.iterations()[*iteration as usize] else {
                    return Err(backend("compact element lacks generator"));
                };
                let Some(element) = generator.element() else {
                    return Ok(true);
                };
                let cursor = self.cursor(*iteration)?;
                let index = b.use_var(cursor.index);
                let count = b.use_var(cursor.count);
                let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, count);
                guard(b, valid, error, arrays::BOUNDS);
                let values = match generator.source() {
                    Source::U64Range(_) => {
                        let start = b.use_var(cursor.start);
                        vec![b.ins().iadd(start, index)]
                    }
                    Source::Value(source) => {
                        let source_ty = layout.ty(source.get()).element()?;
                        let view = arrays::read(words, b, layout, *source);
                        let offset = b
                            .ins()
                            .imul_imm(index, source_ty.packed_bytes.unwrap() as i64);
                        let address = b.ins().iadd(view[0], offset);
                        packed::load(b, address, source_ty)?
                    }
                };
                if values.len() != layout.ty(element.get()).words {
                    return Err(backend(
                        "compact generator element differs from declared result",
                    ));
                }
                arrays::write(words, b, layout.slot(element.get()), &values);
            }
            Action::CollectionPush { collection, value } => {
                let element = layout
                    .ty(plan.collections()[*collection as usize].output().get())
                    .element()?;
                if matches!(
                    plan.collections()[*collection as usize].kind(),
                    ComprehensionKind::MatrixHorizontalBlocks { .. }
                ) {
                    if element.as_ref() != layout.ty(value.get()).element()?.as_ref() {
                        return Err(backend("native block yield element differs"));
                    }
                    let view = self.view(b, *collection);
                    let block = arrays::read(words, b, layout, *value);
                    let known = b.use_var(self.rows_set[*collection as usize]);
                    let rows = b.ins().select(known, view[3], block[3]);
                    let same = b.ins().icmp(IntCC::Equal, rows, block[3]);
                    guard(b, same, error, INVALID);
                    let (length, overflow) = b.ins().uadd_overflow(view[1], block[1]);
                    let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
                    guard(b, valid, error, 1);
                    let (columns, overflow) = b.ins().uadd_overflow(view[4], block[4]);
                    let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
                    guard(b, valid, error, 1);
                    let stride = element.packed_bytes.unwrap();
                    let capacity = Arrays::grown_capacity(b, view[2], length, stride, error);
                    let mut next = arrays.writable(b, &view, true, capacity, element, error)?;
                    let offset = b.ins().imul_imm(view[1], stride as i64);
                    let destination = b.ins().iadd(next[0], offset);
                    let bytes = b.ins().imul_imm(block[1], stride as i64);
                    arrays.copy(b, destination, block[0], bytes);
                    next[1] = length;
                    next[3] = rows;
                    next[4] = columns;
                    b.def_var(self.rows_set[*collection as usize], one);
                    self.replace(b, arrays, *collection, &next, error);
                    return Ok(true);
                }
                if element.as_ref() != layout.ty(value.get()).as_ref() {
                    return Err(backend("compact yield has a different native element type"));
                }
                let view = self.view(b, *collection);
                let values = arrays::read(words, b, layout, *value);
                let next = arrays.append_collection(b, &view, &values, element, error)?;
                self.replace(b, arrays, *collection, &next, error);
            }
            Action::CollectionFinish { collection } => {
                let definition = &plan.collections()[*collection as usize];
                let mut view = self.view(b, *collection);
                if definition.kind() == ComprehensionKind::MatrixPreserveShape {
                    let Loop::Collection(generator) =
                        &plan.iterations()[definition.generators()[0] as usize]
                    else {
                        return Err(backend("compact preserved shape has no generator"));
                    };
                    let Source::Value(source) = generator.source() else {
                        return Err(backend("compact preserved shape requires actual matrix"));
                    };
                    let source = arrays::read(words, b, layout, *source);
                    let valid = b.ins().icmp(IntCC::Equal, view[1], source[1]);
                    guard(b, valid, error, INVALID);
                    view[3] = source[3];
                    view[4] = source[4];
                } else if !matches!(
                    definition.kind(),
                    ComprehensionKind::MatrixHorizontalBlocks { .. }
                ) {
                    view[3] = one;
                    view[4] = view[1];
                }
                arrays::write(words, b, layout.slot(definition.output().get()), &view);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub fn header<M: Module>(
        &self,
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        layout: &Layout,
        arrays: &Arrays,
        prepared: &PreparedNativeProgram,
        initial: &BTreeMap<usize, Initial>,
        kernels: &[Kernel],
        iteration: u32,
        body: Block,
        exit: Block,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        let cursor = self.cursor(iteration)?;
        if let Some(proof) = prepared
            .metadata()
            .uniform_word_collections
            .iter()
            .find(|p| p.iteration() == iteration)
        {
            let word = words.read(b, layout.slot(proof.word().get()));
            let start = b.use_var(cursor.start);
            let declared_start = words.read(b, layout.slot(proof.start().get()));
            let fresh = b.ins().icmp(IntCC::Equal, start, declared_start);
            guard(b, fresh, error, INVALID);
            let actual = b.use_var(cursor.count);
            let count = if proof.filtered() {
                let count = words.read(b, layout.slot(proof.limit().get()));
                let expected = b.ins().iadd_imm(count, 1);
                let fresh = b.ins().icmp(IntCC::Equal, expected, actual);
                guard(b, fresh, error, INVALID);
                count
            } else {
                actual
            };
            let index = b.use_var(cursor.index);
            let fresh = b.ins().icmp_imm(IntCC::Equal, index, 0);
            guard(b, fresh, error, INVALID);
            let old = self.view(b, proof.collection());
            for value in [old[1], old[2], old[5]] {
                let fresh = b.ins().icmp_imm(IntCC::Equal, value, 0);
                guard(b, fresh, error, INVALID);
            }
            let element = layout
                .ty(prepared.plan().collections()[proof.collection() as usize]
                    .output()
                    .get())
                .element()?;
            let zeroed = matches!(
                initial.get(&layout.slot(proof.word().get())),
                Some(Initial::Bits(0))
            );
            let mut view = arrays.allocate(b, count, element, zeroed, error)?;
            if !zeroed {
                let zero = b.ins().iconst(types::I64, 0);
                let index = variable(b, zero);
                let header = b.create_block();
                let fill = b.create_block();
                let done = b.create_block();
                b.ins().jump(header, &[]);
                b.switch_to_block(header);
                let at = b.use_var(index);
                let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
                b.ins().brif(more, fill, &[], done, &[]);
                b.switch_to_block(fill);
                let offset = b.ins().imul_imm(at, element.packed_bytes.unwrap() as i64);
                let address = b.ins().iadd(view[0], offset);
                packed::store(b, address, element, &[word])?;
                let next = b.ins().iadd_imm(at, 1);
                b.def_var(index, next);
                b.ins().jump(header, &[]);
                b.switch_to_block(done);
            }
            view[1] = count;
            view[4] = count;
            self.replace(b, arrays, proof.collection(), &view, error);
            b.def_var(cursor.index, actual);
            b.ins().jump(exit, &[]);
        } else {
            if let Some(kernel) = kernels.iter().find(|k| k.proof.iteration == iteration) {
                let index = b.use_var(cursor.index);
                let count = b.use_var(cursor.count);
                let old = self.view(b, kernel.proof.collection);
                let source = (0..6)
                    .map(|lane| words.read(b, layout.slot(kernel.proof.source) + lane))
                    .collect::<Vec<_>>();
                let mut fresh = b.ins().icmp_imm(IntCC::Equal, index, 0);
                for value in [old[1], old[2], old[5]] {
                    let empty = b.ins().icmp_imm(IntCC::Equal, value, 0);
                    fresh = b.ins().band(fresh, empty);
                }
                let same = b.ins().icmp(IntCC::Equal, count, source[1]);
                fresh = b.ins().band(fresh, same);
                let fast = b.create_block();
                let ordinary = b.create_block();
                b.ins().brif(fresh, fast, &[], ordinary, &[]);
                b.switch_to_block(fast);
                let fill = b.create_block();
                b.ins().brif(count, fill, &[], exit, &[]);
                b.switch_to_block(fill);
                // This is the existing collection result capacity, including
                // its minimum-four and power-of-two growth geometry. Total
                // scalar bodies permit one physical allocation, as in the
                // ordinary admitted collection; no source error is advanced.
                let four = b.ins().iconst(types::I64, 4);
                let minimum = b.ins().umax(count, four);
                let prior = b.ins().iadd_imm(minimum, -1);
                let leading = b.ins().clz(prior);
                let bits = b.ins().iconst(types::I64, 64);
                let shift = b.ins().isub(bits, leading);
                let fits = b.ins().icmp_imm(IntCC::UnsignedLessThan, shift, 64);
                guard(b, fits, error, 1);
                let one = b.ins().iconst(types::I64, 1);
                let capacity = b.ins().ishl(one, shift);
                let element = layout
                    .ty(
                        prepared.plan().collections()[kernel.proof.collection as usize]
                            .output()
                            .get(),
                    )
                    .element()?;
                if element.packed_bytes != Some(kernel.proof.output_width.into()) {
                    return Err(backend("compact word sink differs from its total proof"));
                }
                let mut view = arrays.allocate(b, capacity, element, false, error)?;
                kernel.emit(module, b, words, layout, source[0], view[0], count, error)?;
                view[1] = count;
                view[4] = count;
                self.replace(b, arrays, kernel.proof.collection, &view, error);
                b.def_var(cursor.index, count);
                b.ins().jump(exit, &[]);
                b.switch_to_block(ordinary);
            }
            let index = b.use_var(cursor.index);
            let count = b.use_var(cursor.count);
            let more = b.ins().icmp(IntCC::UnsignedLessThan, index, count);
            b.ins().brif(more, body, &[], exit, &[]);
        }
        Ok(())
    }
}
