//! Native sorted String-key Maps. Comparisons, binary searches, insertion and
//! histogram arithmetic are generated instructions. Imported functions only
//! allocate/COW backing storage, move initialized bytes, or free storage.
use super::{
    arrays::{self, Arrays},
    layout::{Kind, Layout},
    packed, strings,
};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, guard},
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{NativeCollectionPrimitive as P, NativeMatchValue as V};

pub(super) fn supports(op: P) -> bool {
    matches!(
        op,
        P::MapNew
            | P::MapInsert
            | P::MapContainsKey
            | P::MapKeyAccess
            | P::StringHistogram
            | P::StringMapKeys
            | P::StringMapValues
    )
}
pub(super) fn add(b: &mut FunctionBuilder<'_>, a: Value, c: Value, error: Block) -> Value {
    let (sum, overflow) = b.ins().uadd_overflow(a, c);
    let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    guard(b, valid, error, 1);
    sum
}
fn address(b: &mut FunctionBuilder<'_>, pointer: Value, ordinal: Value, stride: usize) -> Value {
    let offset = b.ins().imul_imm(ordinal, stride as i64);
    b.ins().iadd(pointer, offset)
}
/// Exact lower_bound plus presence. The source invariant is canonical sorted,
/// unique UTF8 keys, maintained by every construction and insertion below.
pub(super) fn find(
    b: &mut FunctionBuilder<'_>,
    pointer: Value,
    length: Value,
    key: &[Value],
    stride: usize,
    endian: cranelift_codegen::ir::Endianness,
) -> (Value, Value) {
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let search = b.create_block();
    b.append_block_param(search, types::I64);
    b.append_block_param(search, types::I64);
    let compare = b.create_block();
    let lower = b.create_block();
    let upper = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::I64);
    b.append_block_param(done, types::I64);
    b.ins().jump(search, &[zero.into(), length.into()]);
    b.switch_to_block(search);
    let lo = b.block_params(search)[0];
    let hi = b.block_params(search)[1];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, lo, hi);
    b.ins()
        .brif(more, compare, &[], done, &[lo.into(), zero.into()]);
    b.switch_to_block(compare);
    let span = b.ins().isub(hi, lo);
    let half = b.ins().ushr_imm(span, 1);
    let mid = b.ins().iadd(lo, half);
    let at = address(b, pointer, mid, stride);
    let text = b.ins().load(types::I64, MemFlags::new(), at, 0);
    let len = b.ins().load(types::I64, MemFlags::new(), at, 8);
    let order = strings::order(b, key[0], key[1], text, len, endian);
    let same = b.ins().icmp_imm(IntCC::Equal, order, 0);
    let different = b.create_block();
    b.ins()
        .brif(same, done, &[mid.into(), one.into()], different, &[]);
    b.switch_to_block(different);
    let less = b.ins().icmp_imm(IntCC::SignedLessThan, order, 0);
    b.ins().brif(less, upper, &[], lower, &[]);
    b.switch_to_block(upper);
    b.ins().jump(search, &[lo.into(), mid.into()]);
    b.switch_to_block(lower);
    let next = b.ins().iadd_imm(mid, 1);
    b.ins().jump(search, &[next.into(), hi.into()]);
    b.switch_to_block(done);
    (b.block_params(done)[0], b.block_params(done)[1])
}
fn insert_space(
    arrays: &Arrays,
    b: &mut FunctionBuilder<'_>,
    view: &[Value],
    at: Value,
    present: Value,
    stride: usize,
) {
    let insert = b.create_block();
    let done = b.create_block();
    b.ins().brif(present, done, &[], insert, &[]);
    b.switch_to_block(insert);
    let source = address(b, view[0], at, stride);
    let destination = b.ins().iadd_imm(source, stride as i64);
    let remaining = b.ins().isub(view[1], at);
    let bytes = b.ins().imul_imm(remaining, stride as i64);
    arrays.copy(b, destination, source, bytes);
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
}
fn arity(operands: &[V], expected: usize) -> Result<(), NativeCompileError> {
    if operands.len() != expected {
        return Err(backend("native Map operation arity differs"));
    }
    Ok(())
}
pub(super) fn emit(
    arrays: &Arrays,
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    layout: &Layout,
    consume: bool,
    op: P,
    operands: &[V],
    output: V,
    error: Block,
    endian: cranelift_codegen::ir::Endianness,
    histogram: Option<cranelift_codegen::ir::FuncRef>,
    classes: Option<(cranelift_codegen::ir::FuncRef, &super::utf8_classes::State)>,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let out = layout.ty(output.get());
    if op == P::MapNew {
        arity(operands, 0)?;
        out.map_entry()?;
        arrays::write(
            words,
            b,
            layout.slot(output.get()),
            &[zero, zero, zero, zero, zero, zero],
        );
        return Ok(());
    }
    if op == P::StringHistogram {
        arity(operands, 1)?;
        if !matches!(layout.ty(operands[0].get()).element()?.kind, Kind::Utf8) {
            return Err(backend(
                "native String histogram requires immutable UTF8 elements",
            ));
        }
        let entry = out.map_entry()?;
        let fields = entry.fields()?;
        if !matches!(
            fields[1].ty.kind,
            Kind::Scalar {
                bytes: 8,
                signed: false,
                float: false
            }
        ) {
            return Err(backend("native String histogram requires U64 counts"));
        }
        super::histogram::emit(
            arrays,
            b,
            words,
            layout,
            operands[0],
            output,
            error,
            endian,
            classes
                .map(|(function, _)| function)
                .or(histogram)
                .ok_or_else(|| backend("native histogram leaf absent"))?,
            classes.map(|(_, state)| state),
        )?;
        return Ok(());
    }
    arity(
        operands,
        match op {
            P::MapInsert => 3,
            P::StringMapKeys | P::StringMapValues => 1,
            _ => 2,
        },
    )?;
    let entry = layout.ty(operands[0].get()).map_entry()?;
    let fields = entry.fields()?;
    let stride = entry.packed_bytes.unwrap();
    let value_at = fields[1].packed_offset.unwrap();
    let source = arrays::read(words, b, layout, operands[0]);
    if matches!(op, P::StringMapKeys | P::StringMapValues) {
        let selected = if op == P::StringMapKeys { 0 } else { 1 };
        let element = out.element()?;
        if **element != *fields[selected].ty {
            return Err(backend("native Map projection result differs"));
        }
        let offset = fields[selected].packed_offset.unwrap();
        let output_stride = element.packed_bytes.unwrap();
        let mut view = arrays.allocate(b, source[1], element, false, error)?;
        let header = b.create_block();
        b.append_block_param(header, types::I64);
        let body = b.create_block();
        let done = b.create_block();
        b.ins().jump(header, &[zero.into()]);
        b.switch_to_block(header);
        let i = b.block_params(header)[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, i, source[1]);
        b.ins().brif(more, body, &[], done, &[]);
        b.switch_to_block(body);
        let at = address(b, source[0], i, stride);
        let at = b.ins().iadd_imm(at, offset as i64);
        let value = packed::load(b, at, element)?;
        let at = address(b, view[0], i, output_stride);
        packed::store(b, at, element, &value)?;
        let next = b.ins().iadd_imm(i, 1);
        b.ins().jump(header, &[next.into()]);
        b.switch_to_block(done);
        view[1] = source[1];
        view[3] = one;
        view[4] = source[1];
        arrays::write(words, b, layout.slot(output.get()), &view);
        return Ok(());
    }
    if !matches!(layout.ty(operands[1].get()).kind, Kind::Utf8) {
        return Err(backend("native Map key requires UTF8"));
    }
    let key = arrays::read(words, b, layout, operands[1]);
    let (at, present) = if op == P::MapInsert {
        find(b, source[0], source[1], &key, stride, endian)
    } else {
        super::map_index::find(b, &source, &key, stride, endian, error)
    };
    match op {
        P::MapContainsKey => {
            arrays::write(words, b, layout.slot(output.get()), &[present]);
        }
        P::MapKeyAccess => {
            if **out != *fields[1].ty {
                return Err(backend("native Map access result differs"));
            }
            guard(b, present, error, 0x19501);
            let ptr = address(b, source[0], at, stride);
            let ptr = b.ins().iadd_imm(ptr, value_at as i64);
            let values = packed::load(b, ptr, out)?;
            arrays::write(words, b, layout.slot(output.get()), &values);
        }
        P::MapInsert => {
            if **out != **layout.ty(operands[0].get())
                || **layout.ty(operands[2].get()) != *fields[1].ty
            {
                return Err(backend("native Map insertion types differ"));
            }
            let missing = b.ins().bxor_imm(present, 1);
            let length = add(b, source[1], missing, error);
            let capacity = Arrays::grown_capacity(b, source[2], length, stride, error);
            let mut view =
                super::map_index::writable(arrays, b, &source, consume, capacity, entry, error)?;
            insert_space(arrays, b, &view, at, present, stride);
            let ptr = address(b, view[0], at, stride);
            let mut values = key;
            values.extend(arrays::read(words, b, layout, operands[2]));
            packed::store(b, ptr, entry, &values)?;
            view[1] = length;
            super::map_index::rebuild(b, &view, entry, error);
            arrays::write(words, b, layout.slot(output.get()), &view);
        }
        _ => return Err(backend("unselected native Map operation")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::AbiParam;
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};
    #[repr(C)]
    #[derive(Debug)]
    struct Found {
        index: u64,
        present: u64,
    }
    type Lookup = unsafe extern "C" fn(*const [u64; 3], usize, *const u8, usize) -> Found;
    #[test]
    fn native_sorted_utf8_lookup_preserves_canonical_order_and_empty_spans() {
        let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
        let endian = module.isa().endianness();
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 4];
        context.func.signature.returns = vec![AbiParam::new(types::I64); 2];
        let mut fb = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let a = b.block_params(entry).to_vec();
            let (index, present) = find(&mut b, a[0], a[1], &a[2..4], 24, endian);
            b.ins().return_(&[index, present]);
            b.seal_all_blocks();
            b.finalize();
        }
        assert!(context.func.layout.blocks().all(|block| {
            context
                .func
                .layout
                .block_insts(block)
                .all(|inst| !context.func.dfg.insts[inst].opcode().is_call())
        }));
        let id = module
            .declare_function(
                "native_sorted_utf8_lookup",
                Linkage::Export,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: exact four-word native input and two-word C result above;
        // the retained module owns the executable code throughout every call.
        let lookup: Lookup = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
        let mut keys = [
            "",
            "a",
            "a\0b",
            "é",
            "e\u{301}",
            "🦀",
            "\r\n",
            "aaaazzzzz",
            "aaabaaaaa",
            "abcdefghijklmno",
            "abcdefghijklmnp",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        keys.extend((0..40).map(|i| format!("{}{:02}🦀", "a".repeat(i), i)));
        keys.sort();
        keys.dedup();
        let mut queries = keys.clone();
        queries.extend(["missing".into(), "\0".into(), "abcdefghijklmnn".into()]);
        for length in 0..=keys.len() {
            let entries = keys[..length]
                .iter()
                .map(|key| [key.as_ptr() as u64, key.len() as u64, 17])
                .collect::<Vec<_>>();
            for query in &queries {
                // SAFETY: all entry/key spans remain live and immutable. Empty
                // keys deliberately admit a null pointer with a zero extent.
                let actual = unsafe {
                    lookup(
                        entries.as_ptr(),
                        entries.len(),
                        if query.is_empty() {
                            std::ptr::null()
                        } else {
                            query.as_ptr()
                        },
                        query.len(),
                    )
                };
                let expected = keys[..length].partition_point(|key| key < query);
                assert_eq!(actual.index, expected as u64, "{length} {query:?}");
                assert_eq!(
                    actual.present,
                    u64::from(keys[..length].get(expected) == Some(query)),
                    "{length} {query:?}"
                );
            }
        }
    }
}
