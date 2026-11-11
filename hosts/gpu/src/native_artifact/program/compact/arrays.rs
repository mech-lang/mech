//! Concrete typed array operations. Calls are allocation, initialized copying
//! and final deallocation, never canonical binding or per-operation dispatch.
use super::{
    layout::{Kind, Layout, Type},
    packed,
};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, guard, offset},
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    AbiParam, Block, FuncRef, InstBuilder, MemFlags, StackSlot, StackSlotData, StackSlotKind,
    Value, types,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{Linkage, Module};
use mech_engine::native_artifact::{NativeCollectionPrimitive as P, NativeMatchValue as V};

pub(super) const BOUNDS: u64 = 0x19401;
const INVALID: u64 = 0x19403;
pub(in crate::native_artifact) struct Arrays {
    allocate: FuncRef,
    writable: FuncRef,
    free: FuncRef,
    memmove: FuncRef,
    source: StackSlot,
    result: StackSlot,
    pub heap: Value,
    retirement_observer: Option<FuncRef>,
}
fn import<M: Module>(
    module: &mut M,
    b: &mut FunctionBuilder<'_>,
    name: &str,
    parameters: usize,
) -> Result<FuncRef, NativeCompileError> {
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); parameters];
    signature.returns = vec![AbiParam::new(types::I64)];
    let id = module
        .declare_function(name, Linkage::Import, &signature)
        .map_err(backend)?;
    Ok(module.declare_func_in_func(id, b.func))
}
fn success(b: &mut FunctionBuilder<'_>, status: Value, error: Block) {
    let next = b.create_block();
    b.ins().brif(status, error, &[status.into()], next, &[]);
    b.switch_to_block(next);
}
fn checked_add(b: &mut FunctionBuilder<'_>, left: Value, right: Value, error: Block) -> Value {
    let (sum, overflow) = b.ins().uadd_overflow(left, right);
    let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    guard(b, valid, error, 1);
    sum
}
pub(super) fn read(
    words: &Words,
    b: &mut FunctionBuilder<'_>,
    layout: &Layout,
    value: V,
) -> Vec<Value> {
    (0..layout.ty(value.get()).words)
        .map(|i| words.read(b, layout.slot(value.get()) + i))
        .collect()
}
pub(super) fn write(words: &Words, b: &mut FunctionBuilder<'_>, base: usize, values: &[Value]) {
    for (i, &value) in values.iter().enumerate() {
        words.write(b, base + i, value)
    }
}
fn row(b: &mut FunctionBuilder<'_>, view: &[Value], error: Block) {
    let valid = b.ins().icmp_imm(IntCC::Equal, view[3], 1);
    guard(b, valid, error, INVALID);
}
fn same_elements(left: &Type, right: &Type) -> Result<(), NativeCompileError> {
    if left != right {
        return Err(backend("compact array operation element layouts differ"));
    }
    Ok(())
}
fn unsigned(ty: &Type) -> Result<(), NativeCompileError> {
    match ty.kind {
        Kind::Scalar {
            signed: false,
            float: false,
            ..
        } => Ok(()),
        _ => Err(backend(
            "compact sequence primitive requires the selected unsigned element family",
        )),
    }
}
fn integer_element(ty: &Type) -> bool {
    match &ty.kind {
        Kind::Scalar { float: false, .. } => true,
        Kind::Product(fields) => fields.iter().all(|f| integer_element(&f.ty)),
        _ => false,
    }
}
impl Arrays {
    pub fn new<M: Module>(
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        heap: Value,
        observe: bool,
    ) -> Result<Self, NativeCompileError> {
        Ok(Self {
            allocate: import(module, b, "mech_compact_allocate", 6)?,
            writable: import(module, b, "mech_compact_writable", 7)?,
            free: import(module, b, "mech_compact_free", 2)?,
            memmove: import(module, b, "memmove", 3)?,
            source: b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                48,
                3,
            )),
            result: b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                48,
                3,
            )),
            heap,
            retirement_observer: if observe {
                Some(import(module, b, "mech_compact_phase_observe", 3)?)
            } else {
                None
            },
        })
    }
    pub fn retain(&self, b: &mut FunctionBuilder<'_>, owner: Value) {
        let yes = b.create_block();
        let done = b.create_block();
        b.ins().brif(owner, yes, &[], done, &[]);
        b.switch_to_block(yes);
        let refs = b.ins().load(types::I64, MemFlags::new(), owner, 0);
        let refs = b.ins().iadd_imm(refs, 1);
        b.ins().store(MemFlags::new(), refs, owner, 0);
        b.ins().jump(done, &[]);
        b.switch_to_block(done);
    }
    pub fn release(&self, b: &mut FunctionBuilder<'_>, owner: Value, error: Block) {
        let yes = b.create_block();
        let deallocate = b.create_block();
        let done = b.create_block();
        b.ins().brif(owner, yes, &[], done, &[]);
        b.switch_to_block(yes);
        let refs = b.ins().load(types::I64, MemFlags::new(), owner, 0);
        let refs = b.ins().iadd_imm(refs, -1);
        b.ins().store(MemFlags::new(), refs, owner, 0);
        b.ins().brif(refs, done, &[], deallocate, &[]);
        b.switch_to_block(deallocate);
        let call = b.ins().call(self.free, &[self.heap, owner]);
        let status = b.inst_results(call)[0];
        success(b, status, error);
        b.ins().jump(done, &[]);
        b.switch_to_block(done);
    }
    pub fn retire(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        slots: &[usize],
        error: Block,
    ) {
        for &at in slots {
            let owner = words.read(b, at);
            let zero = b.ins().iconst(types::I64, 0);
            words.write(b, at, zero);
            self.release(b, owner, error);
        }
    }
    fn result(&self, b: &mut FunctionBuilder<'_>) -> Vec<Value> {
        (0..6)
            .map(|i| b.ins().stack_load(types::I64, self.result, (i * 8) as i32))
            .collect()
    }
    pub(super) fn allocate(
        &self,
        b: &mut FunctionBuilder<'_>,
        capacity: Value,
        element: &Type,
        zeroed: bool,
        error: Block,
    ) -> Result<Vec<Value>, NativeCompileError> {
        let stride = element
            .packed_bytes
            .ok_or_else(|| backend("compact allocator requires a fixed element stride"))?;
        let stride = b.ins().iconst(types::I64, stride as i64);
        let align = b.ins().iconst(types::I64, element.packed_align as i64);
        let zeroed = b.ins().iconst(types::I64, i64::from(zeroed));
        let out = b.ins().stack_addr(types::I64, self.result, 0);
        let call = b.ins().call(
            self.allocate,
            &[self.heap, capacity, stride, align, zeroed, out],
        );
        let status = b.inst_results(call)[0];
        success(b, status, error);
        Ok(self.result(b))
    }
    pub(super) fn writable(
        &self,
        b: &mut FunctionBuilder<'_>,
        view: &[Value],
        consume: bool,
        minimum: Value,
        element: &Type,
        error: Block,
    ) -> Result<Vec<Value>, NativeCompileError> {
        let done = b.create_block();
        for _ in 0..6 {
            b.append_block_param(done, types::I64);
        }
        let fallback = b.create_block();
        if consume {
            let inspect = b.create_block();
            b.ins().brif(view[5], inspect, &[], fallback, &[]);
            b.switch_to_block(inspect);
            let references = b.ins().load(types::I64, MemFlags::new(), view[5], 0);
            let unique = b.ins().icmp_imm(IntCC::Equal, references, 1);
            let fits = b
                .ins()
                .icmp(IntCC::UnsignedLessThanOrEqual, minimum, view[2]);
            let reuse = b.ins().band(unique, fits);
            b.ins().brif(
                reuse,
                done,
                &view.iter().copied().map(Into::into).collect::<Vec<_>>(),
                fallback,
                &[],
            );
        } else {
            b.ins().jump(fallback, &[]);
        }
        b.switch_to_block(fallback);
        let stride = element
            .packed_bytes
            .ok_or_else(|| backend("compact writable backing requires a fixed element stride"))?;
        let stride = b.ins().iconst(types::I64, stride as i64);
        let align = b.ins().iconst(types::I64, element.packed_align as i64);
        let consume = b.ins().iconst(types::I64, i64::from(consume));
        for (i, &value) in view.iter().enumerate() {
            b.ins().stack_store(value, self.source, (i * 8) as i32);
        }
        let source = b.ins().stack_addr(types::I64, self.source, 0);
        let out = b.ins().stack_addr(types::I64, self.result, 0);
        let call = b.ins().call(
            self.writable,
            &[self.heap, source, consume, minimum, stride, align, out],
        );
        let status = b.inst_results(call)[0];
        success(b, status, error);
        let result = self.result(b);
        b.ins().jump(
            done,
            &result.iter().copied().map(Into::into).collect::<Vec<_>>(),
        );
        b.switch_to_block(done);
        Ok(b.block_params(done).to_vec())
    }
    pub(in crate::native_artifact) fn grow_numeric(
        &self,
        b: &mut FunctionBuilder<'_>,
        view: &[Value],
        width: u8,
        error: Block,
    ) -> Result<Vec<Value>, NativeCompileError> {
        let integer = match width {
            1 => mech_core::IntegerWidth::W8,
            2 => mech_core::IntegerWidth::W16,
            4 => mech_core::IntegerWidth::W32,
            8 => mech_core::IntegerWidth::W64,
            _ => {
                return Err(backend(
                    "compact numeric growth requires an unsigned element",
                ));
            }
        };
        let element = Type::derive(&mech_core::SchemaBody::UnsignedInteger(integer))?;
        let one = b.ins().iconst(types::I64, 1);
        let needed = checked_add(b, view[1], one, error);
        let capacity = Self::grown_capacity(b, view[2], needed, width as usize, error);
        let result = self.writable(b, view, true, capacity, &element, error)?;
        // The phase owns the unique caller reference during this synchronous
        // call. Establish the replacement before releasing its previous owner.
        self.retain(b, result[5]);
        self.release(b, view[5], error);
        Ok(result)
    }
    pub fn copy(
        &self,
        b: &mut FunctionBuilder<'_>,
        destination: Value,
        source: Value,
        bytes: Value,
    ) {
        let yes = b.create_block();
        let done = b.create_block();
        b.ins().brif(bytes, yes, &[], done, &[]);
        b.switch_to_block(yes);
        b.ins().call(self.memmove, &[destination, source, bytes]);
        b.ins().jump(done, &[]);
        b.switch_to_block(done);
    }
    pub(super) fn grown_capacity(
        b: &mut FunctionBuilder<'_>,
        capacity: Value,
        needed: Value,
        width: usize,
        error: Block,
    ) -> Value {
        let enough = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, needed, capacity);
        let grow = b.create_block();
        let done = b.create_block();
        b.append_block_param(done, types::I64);
        b.ins().brif(enough, done, &[capacity.into()], grow, &[]);
        b.switch_to_block(grow);
        let initial = b.create_block();
        let nonzero = b.create_block();
        let grown = b.create_block();
        b.append_block_param(grown, types::I64);
        b.ins().brif(capacity, nonzero, &[], initial, &[]);
        b.switch_to_block(initial);
        let first = b.ins().iconst(types::I64, (64 / width) as i64);
        b.ins().jump(grown, &[first.into()]);
        b.switch_to_block(nonzero);
        let small = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThan, capacity, (4096 / width) as i64);
        let large = b.ins().icmp_imm(
            IntCC::UnsignedGreaterThan,
            capacity,
            (131072 / width) as i64,
        );
        let double = b.ins().bor(small, large);
        let twice = b.create_block();
        let middle = b.create_block();
        b.ins().brif(double, twice, &[], middle, &[]);
        b.switch_to_block(twice);
        let n = checked_add(b, capacity, capacity, error);
        b.ins().jump(grown, &[n.into()]);
        b.switch_to_block(middle);
        let twice = checked_add(b, capacity, capacity, error);
        let triple = checked_add(b, twice, capacity, error);
        let one = b.ins().iconst(types::I64, 1);
        let rounded = checked_add(b, triple, one, error);
        let n = b.ins().ushr_imm(rounded, 1);
        b.ins().jump(grown, &[n.into()]);
        b.switch_to_block(grown);
        let growth = b.block_params(grown)[0];
        let capacity = b.ins().umax(growth, needed);
        b.ins().jump(done, &[capacity.into()]);
        b.switch_to_block(done);
        b.block_params(done)[0]
    }
    pub(super) fn append_values(
        &self,
        b: &mut FunctionBuilder<'_>,
        view: &[Value],
        values: &[Value],
        element: &Type,
        consume: bool,
        error: Block,
    ) -> Result<Vec<Value>, NativeCompileError> {
        let one = b.ins().iconst(types::I64, 1);
        let length = checked_add(b, view[1], one, error);
        let capacity =
            Self::grown_capacity(b, view[2], length, element.packed_bytes.unwrap(), error);
        self.append_at_capacity(b, view, values, element, consume, length, capacity, error)
    }
    fn append_at_capacity(
        &self,
        b: &mut FunctionBuilder<'_>,
        view: &[Value],
        values: &[Value],
        element: &Type,
        consume: bool,
        length: Value,
        capacity: Value,
        error: Block,
    ) -> Result<Vec<Value>, NativeCompileError> {
        let mut out = self.writable(b, view, consume, capacity, element, error)?;
        let bytes = b
            .ins()
            .imul_imm(view[1], element.packed_bytes.unwrap() as i64);
        let address = b.ins().iadd(out[0], bytes);
        packed::store(b, address, element, values)?;
        out[1] = length;
        out[3] = b.ins().iconst(types::I64, 1);
        out[4] = length;
        Ok(out)
    }
    pub(super) fn append_collection(
        &self,
        b: &mut FunctionBuilder<'_>,
        view: &[Value],
        values: &[Value],
        element: &Type,
        error: Block,
    ) -> Result<Vec<Value>, NativeCompileError> {
        let one = b.ins().iconst(types::I64, 1);
        let length = checked_add(b, view[1], one, error);
        let grow = b.ins().icmp(IntCC::UnsignedGreaterThan, length, view[2]);
        let yes = b.create_block();
        let done = b.create_block();
        b.append_block_param(done, types::I64);
        b.ins().brif(grow, yes, &[], done, &[view[2].into()]);
        b.switch_to_block(yes);
        let twice = checked_add(b, view[2], view[2], error);
        let four = b.ins().iconst(types::I64, 4);
        let capacity = b.ins().umax(twice, four);
        b.ins().jump(done, &[capacity.into()]);
        b.switch_to_block(done);
        let capacity = b.block_params(done)[0];
        self.append_at_capacity(b, view, values, element, true, length, capacity, error)
    }
    pub(super) fn emit(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        layout: &Layout,
        source_schema: &mech_core::SchemaBody,
        consume: bool,
        retire_before_change: &[usize],
        site: usize,
        private: Option<&super::private_chains::Admitted<'_>>,
        operation: P,
        operands: &[V],
        output: V,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        let ty = layout.ty(output.get());
        let element = ty.element()?;
        let width = element.packed_bytes.unwrap();
        let base = layout.slot(output.get());
        let one = b.ins().iconst(types::I64, 1);
        let zero = b.ins().iconst(types::I64, 0);
        let expected = match operation {
            P::SequenceClear | P::MatrixHorizontal(1) | P::MatrixConversion => 1,
            P::SequenceWithCapacity
            | P::SequenceReserve
            | P::SequenceAppend
            | P::MatrixHorizontal(2) => 2,
            P::SequenceAppendLeBytes | P::MatrixIndexedReplace => 3,
            P::SequenceAppendSublist => 4,
            _ => return Err(backend("compact unresolved concrete array operation")),
        };
        if operands.len() != expected {
            return Err(backend("compact concrete array source arity mismatch"));
        }
        if operation == P::SequenceWithCapacity {
            unsigned(element)?;
            same_elements(element, layout.ty(operands[0].get()))?;
            let capacity = words.read(b, layout.slot(operands[1].get()));
            let out = self.allocate(b, capacity, element, false, error)?;
            write(words, b, base, &out);
            return Ok(());
        }
        if operation == P::MatrixHorizontal(1) {
            same_elements(element, layout.ty(operands[0].get()))?;
            let values = read(words, b, layout, operands[0]);
            let mut out = self.allocate(b, one, element, false, error)?;
            packed::store(b, out[0], element, &values)?;
            out[1] = one;
            out[4] = one;
            write(words, b, base, &out);
            return Ok(());
        }
        // A horizontal constructor may receive two scalar elements, including
        // fixed products containing UTF8 spans. These are elements of the
        // output matrix, rather than pre-existing array views.
        if operation == P::MatrixHorizontal(2)
            && operands
                .iter()
                .all(|value| layout.ty(value.get()).as_ref() == element.as_ref())
        {
            let two = b.ins().iconst(types::I64, 2);
            let mut out = self.allocate(b, two, element, false, error)?;
            for (ordinal, &value) in operands.iter().enumerate() {
                let address = b.ins().iadd_imm(out[0], (ordinal * width) as i64);
                let values = read(words, b, layout, value);
                packed::store(b, address, element, &values)?;
            }
            out[1] = two;
            out[4] = two;
            write(words, b, base, &out);
            return Ok(());
        }
        let source = operands[0];
        same_elements(element, layout.ty(source.get()).element()?)?;
        let view = read(words, b, layout, source);
        let out = match operation {
            P::SequenceClear => {
                unsigned(element)?;
                row(b, &view, error);
                let mut out = view;
                out[1] = zero;
                out[3] = one;
                out[4] = zero;
                out
            }
            P::MatrixConversion => {
                row(b, &view, error);
                view
            }
            P::SequenceReserve => {
                unsigned(element)?;
                row(b, &view, error);
                let additional = words.read(b, layout.slot(operands[1].get()));
                let (sum, overflow) = b.ins().uadd_overflow(view[1], additional);
                let max = b.ins().iconst(types::I64, -1);
                let minimum = b.ins().select(overflow, max, sum);
                let need = b.ins().icmp(IntCC::UnsignedGreaterThan, minimum, view[2]);
                let grow = b.create_block();
                let done = b.create_block();
                for _ in 0..6 {
                    b.append_block_param(done, types::I64);
                }
                b.ins().brif(
                    need,
                    grow,
                    &[],
                    done,
                    &view.iter().copied().map(Into::into).collect::<Vec<_>>(),
                );
                b.switch_to_block(grow);
                let out = self.writable(b, &view, consume, minimum, element, error)?;
                b.ins().jump(
                    done,
                    &out.iter().copied().map(Into::into).collect::<Vec<_>>(),
                );
                b.switch_to_block(done);
                b.block_params(done).to_vec()
            }
            P::SequenceAppend => {
                unsigned(element)?;
                row(b, &view, error);
                same_elements(element, layout.ty(operands[1].get()))?;
                let values = read(words, b, layout, operands[1]);
                self.append_values(b, &view, &values, element, consume, error)?
            }
            P::MatrixIndexedReplace => {
                same_elements(element, layout.ty(operands[1].get()))?;
                if !integer_element(element) {
                    return Err(backend(
                        "compact floating indexed equality remains unsupported",
                    ));
                }
                let index = words.read(b, layout.slot(operands[2].get()));
                let index = b.ins().iadd_imm(index, -1);
                let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, view[1]);
                guard(b, valid, error, BOUNDS);
                let physical = super::super::codegen::numeric_words::linear_ordinal(
                    b,
                    index,
                    view[3],
                    view[4],
                    source_schema,
                );
                let bytes = b.ins().imul_imm(physical, width as i64);
                let address = b.ins().iadd(view[0], bytes);
                let values = read(words, b, layout, operands[1]);
                if private.is_some_and(|proof| proof.permits(site, source, output)) {
                    // The enclosing interval has already selected its current
                    // sole-owner guard. Exact element, original bounds and
                    // current-schema translation above are unchanged. No
                    // capacity or identity changes, aliases, or prechange
                    // retirement may occur inside this capability's interval.
                    packed::store(b, address, element, &values)?;
                    write(words, b, base, &view);
                    return Ok(());
                }
                let old = packed::load(b, address, element)?;
                let mut same = b.ins().iconst(types::I8, 1);
                for (&a, &z) in old.iter().zip(&values) {
                    let equal = b.ins().icmp(IntCC::Equal, a, z);
                    same = b.ins().band(same, equal);
                }
                let change = b.create_block();
                let done = b.create_block();
                for _ in 0..6 {
                    b.append_block_param(done, types::I64);
                }
                b.ins().brif(
                    same,
                    done,
                    &view.iter().copied().map(Into::into).collect::<Vec<_>>(),
                    change,
                    &[],
                );
                b.switch_to_block(change);
                // Bounds, original value loads and unchanged-result selection
                // precede any retirement. No source data is modified here.
                for &slot in retire_before_change {
                    let owner = words.read(b, slot);
                    if let Some(observer) = self.retirement_observer {
                        let selected = b.ins().icmp_imm(IntCC::NotEqual, owner, 0);
                        let selected = b.ins().uextend(types::I64, selected);
                        let site = b.ins().iconst(types::I64, (site as u64 | (1 << 60)) as i64);
                        b.ins().call(observer, &[self.heap, site, selected]);
                    }
                    self.retire(b, words, &[slot], error);
                }
                let out = self.writable(b, &view, consume, view[2], element, error)?;
                let address = b.ins().iadd(out[0], bytes);
                packed::store(b, address, element, &values)?;
                b.ins().jump(
                    done,
                    &out.iter().copied().map(Into::into).collect::<Vec<_>>(),
                );
                b.switch_to_block(done);
                b.block_params(done).to_vec()
            }
            P::SequenceAppendLeBytes => {
                if width != 1 {
                    return Err(backend("compact little-endian append requires U8 elements"));
                }
                unsigned(element)?;
                row(b, &view, error);
                let value = words.read(b, layout.slot(operands[1].get()));
                let count = words.read(b, layout.slot(operands[2].get()));
                let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, count, 8);
                guard(b, valid, error, INVALID);
                let length = checked_add(b, view[1], count, error);
                let capacity = Self::grown_capacity(b, view[2], length, width, error);
                let mut out = self.writable(b, &view, consume, capacity, element, error)?;
                for i in 0..8 {
                    let selected = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, count, i);
                    let put = b.create_block();
                    let next = b.create_block();
                    b.ins().brif(selected, put, &[], next, &[]);
                    b.switch_to_block(put);
                    let value = b.ins().ushr_imm(value, i * 8);
                    let value = b.ins().ireduce(types::I8, value);
                    let at = b.ins().iadd_imm(view[1], i);
                    let address = b.ins().iadd(out[0], at);
                    b.ins().store(MemFlags::new(), value, address, 0);
                    b.ins().jump(next, &[]);
                    b.switch_to_block(next);
                }
                out[1] = length;
                out[3] = one;
                out[4] = length;
                out
            }
            P::SequenceAppendSublist | P::MatrixHorizontal(2) => {
                let other = read(words, b, layout, operands[1]);
                same_elements(element, layout.ty(operands[1].get()).element()?)?;
                row(b, &view, error);
                row(b, &other, error);
                let (start, count) = if operation == P::SequenceAppendSublist {
                    unsigned(element)?;
                    let start = words.read(b, layout.slot(operands[2].get()));
                    let start = b.ins().umin(start, other[1]);
                    let remaining = b.ins().isub(other[1], start);
                    let count = words.read(b, layout.slot(operands[3].get()));
                    let count = b.ins().umin(count, remaining);
                    (start, count)
                } else {
                    (zero, other[1])
                };
                let length = checked_add(b, view[1], count, error);
                let mut out = if operation == P::MatrixHorizontal(2) {
                    let mut out = self.allocate(b, length, element, false, error)?;
                    let bytes = b.ins().imul_imm(view[1], width as i64);
                    self.copy(b, out[0], view[0], bytes);
                    out[1] = view[1];
                    out
                } else {
                    let capacity = Self::grown_capacity(b, view[2], length, width, error);
                    self.writable(b, &view, consume, capacity, element, error)?
                };
                let at = b.ins().imul_imm(view[1], width as i64);
                let dest = b.ins().iadd(out[0], at);
                let at = b.ins().imul_imm(start, width as i64);
                let source = b.ins().iadd(other[0], at);
                let bytes = b.ins().imul_imm(count, width as i64);
                self.copy(b, dest, source, bytes);
                out[1] = length;
                out[3] = one;
                out[4] = length;
                out
            }
            _ => {
                return Err(backend(
                    "compact concrete operation was not exhaustively lowered",
                ));
            }
        };
        write(words, b, base, &out);
        let _ = offset(base)?;
        Ok(())
    }
}
