//! Direct pointer/length operations over proved, invocation-owned String views.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::memory_regions;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::options::OptimizationOptions;
use crate::native_artifact::compiler::{backend, offset};
use crate::native_artifact::program::codegen::hooks::Hooks;
use crate::native_artifact::program::prepare::borrowed::{Layout, RegionLayout};
use crate::native_artifact::program::protocol::GlobalHelper;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{NativeBorrowedStringAction as Action, NativeMatchValue};

mod line;

fn word(
    b: &mut FunctionBuilder,
    locals: Value,
    registers: Option<&Words>,
    index: usize,
    flags: MemFlags,
) -> Result<Value, NativeCompileError> {
    if let Some(words) = registers {
        return Ok(words.read(b, index));
    }
    Ok(b.ins().load(types::I64, flags, locals, offset(index)?))
}
fn pair(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    registers: Option<&Words>,
    layout: &RegionLayout,
    v: NativeMatchValue,
) -> Result<[Value; 2], NativeCompileError> {
    let slot = layout.slot(v);
    Ok([
        word(
            b,
            locals,
            registers,
            slot,
            memory_regions::borrowed(options),
        )?,
        word(
            b,
            locals,
            registers,
            slot + 1,
            memory_regions::borrowed(options),
        )?,
    ])
}
fn put(
    b: &mut FunctionBuilder,
    locals: Value,
    registers: Option<&Words>,
    slot: usize,
    value: Value,
    flags: MemFlags,
) -> Result<(), NativeCompileError> {
    if let Some(words) = registers {
        words.write(b, slot, value);
    } else {
        b.ins().store(flags, value, locals, offset(slot)?);
    }
    Ok(())
}
fn put_view(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    registers: Option<&Words>,
    slot: usize,
    value: Value,
) -> Result<(), NativeCompileError> {
    put(
        b,
        locals,
        registers,
        slot,
        value,
        memory_regions::borrowed(options),
    )
}
pub(in crate::native_artifact) fn prepare(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let Some(Some(region)) = layout.initializers.get(site) else {
        return Ok(());
    };
    prepare_region(
        options,
        b,
        locals,
        &layout.regions[*region],
        site,
        crate::native_artifact::program::protocol::abi::BORROWED_STRINGS_PREPARE,
        hooks,
        error,
    )
}
pub(in crate::native_artifact) fn resume(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &RegionLayout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    prepare_region(
        options,
        b,
        locals,
        layout,
        site,
        crate::native_artifact::program::protocol::abi::PREFIX_BORROWED_RESUME,
        hooks,
        error,
    )
}
fn prepare_region(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &RegionLayout,
    site: usize,
    operation: u64,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(b, site, GlobalHelper::decode(operation), [zero; 3], error);
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    put_view(options, b, locals, None, layout.ready, ready)?;
    // The checked callback verifies capacity and writes every capture, even
    // for a declined representation. All targets are admitted native words.
    for index in 0..layout.capture_words() {
        let word = b
            .ins()
            .load(types::I64, MemFlags::new(), hooks.arguments, offset(index)?);
        put_view(options, b, locals, None, layout.captures + index, word)?;
    }
    Ok(())
}
fn bounds(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    registers: Option<&Words>,
    origin_frame: Option<Value>,
    closed: &crate::native_artifact::program::prepare::tail::Layout,
    valid: Value,
    hooks: &Hooks,
    site: usize,
    args: [Value; 3],
    error: Block,
) -> Result<(), NativeCompileError> {
    let good = b.create_block();
    let fail = b.create_block();
    b.ins().brif(valid, good, &[], fail, &[]);
    b.switch_to_block(fail);
    if let Some(frame) = origin_frame {
        crate::native_artifact::compiler::set_origin(b, frame, site as u64);
    }
    if let Some(words) = registers {
        words.spill_all(b, locals)?;
    }
    crate::native_artifact::program::codegen::tail::failure(
        options, b, locals, closed, site, hooks, error,
    )?;
    hooks.call(b, site, GlobalHelper::BorrowedStringFailure, args, error);
    // A bounds callback must fail. Never continue with an unbound pointer.
    let status = b
        .ins()
        .iconst(types::I64, crate::native_artifact::abi::STATUS_ABI as i64);
    b.ins().jump(error, &[status.into()]);
    b.switch_to_block(good);
    Ok(())
}
/// Emit the native branch, leaving the builder in the ordinary fallback arm.
/// The caller joins BOTH arms before emitting the original CFG terminator.
pub(in crate::native_artifact) fn emit(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    closed: &crate::native_artifact::program::prepare::tail::Layout,
    site: usize,
    hooks: Option<&Hooks>,
    error: Block,
) -> Result<Option<Block>, NativeCompileError> {
    let Some((region, action)) = layout.action(site) else {
        return Ok(None);
    };
    let hooks = hooks.ok_or_else(|| backend("borrowed String operation lacks owner hooks"))?;
    let layout = &layout.regions[region];
    let ready = b.ins().load(
        types::I64,
        memory_regions::borrowed(options),
        locals,
        offset(layout.ready)?,
    );
    let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
    let native = b.create_block();
    let ordinary = b.create_block();
    let joined = b.create_block();
    b.ins().brif(ready, native, &[], ordinary, &[]);
    b.switch_to_block(native);
    emit_native(
        options, b, locals, None, None, layout, closed, site, action, hooks, error,
    )?;
    b.ins().jump(joined, &[]);
    b.switch_to_block(ordinary);
    Ok(Some(joined))
}

/// Emit only the admitted native action. The caller proves readiness once and
/// owns boundary synchronization. Source bytes retain their original checks.
pub(in crate::native_artifact::program) fn emit_native(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    registers: Option<&Words>,
    origin_frame: Option<Value>,
    layout: &RegionLayout,
    closed: &crate::native_artifact::program::prepare::tail::Layout,
    site: usize,
    action: &Action,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    let output = action.output();
    let mut view = None;
    match *action {
        // Its complete semantic use set is represented by FixedRange actions.
        // No fake Matrix handle is created; non-native inputs take the call below.
        Action::ConstantRange { .. } => {}
        Action::MatrixElement { source, index, .. } => {
            let [table, count] = pair(options, b, locals, registers, layout, source)?;
            let first = word(
                b,
                locals,
                registers,
                layout.slot(source) + 2,
                memory_regions::borrowed(options),
            )?;
            let index = word(b, locals, registers, index.get() as usize, MemFlags::new())?;
            // Subtract the genuine global ordinal, never bias the table pointer
            // outside its allocation. Underflow also fails this unsigned bound.
            let offset = b.ins().isub(index, first);
            let valid = b.ins().icmp(IntCC::UnsignedLessThan, offset, count);
            bounds(
                options,
                b,
                locals,
                registers,
                origin_frame,
                closed,
                valid,
                hooks,
                site,
                [zero, zero, index],
                error,
            )?;
            view = Some(line::load(b, table, offset));
        }
        Action::Copy { source, .. } => {
            view = Some(pair(options, b, locals, registers, layout, source)?);
        }
        Action::Length { source, .. } => {
            // Length observes only the second home. Loading an unused pointer
            // here leaves a potentially trapping instruction in the machine
            // code even after its value is discarded. Keep all memory flags,
            // source checks and original prices; do not emit that access.
            let length = word(
                b,
                locals,
                registers,
                layout.slot(source) + 1,
                memory_regions::borrowed(options),
            )?;
            put(
                b,
                locals,
                registers,
                output.get() as usize,
                length,
                MemFlags::new(),
            )?;
        }
        Action::ScalarSelect { source, index, .. } => {
            let [pointer, length] = pair(options, b, locals, registers, layout, source)?;
            let index = word(b, locals, registers, index.get() as usize, MemFlags::new())?;
            let offset = b.ins().iadd_imm(index, -1);
            let valid = b.ins().icmp(IntCC::UnsignedLessThan, offset, length);
            bounds(
                options,
                b,
                locals,
                registers,
                origin_frame,
                closed,
                valid,
                hooks,
                site,
                [pointer, length, index],
                error,
            )?;
            view = Some([b.ins().iadd(pointer, offset), b.ins().iconst(types::I64, 1)]);
        }
        Action::FixedRange {
            source,
            first,
            last,
            ..
        } => {
            let [pointer, length] = pair(options, b, locals, registers, layout, source)?;
            let last_word = b.ins().iconst(types::I64, last as i64);
            let valid = b
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, length, last_word);
            bounds(
                options,
                b,
                locals,
                registers,
                origin_frame,
                closed,
                valid,
                hooks,
                site,
                [pointer, length, zero],
                error,
            )?;
            view = Some([
                b.ins().iadd_imm(pointer, (first - 1) as i64),
                b.ins().iconst(types::I64, (last - first + 1) as i64),
            ]);
        }
        Action::Equal {
            source,
            literal,
            negate,
            ..
        } => {
            let [left, length] = pair(options, b, locals, registers, layout, source)?;
            let [right, right_length] = pair(options, b, locals, registers, layout, literal)?;
            let same = b.ins().icmp(IntCC::Equal, length, right_length);
            let header = b.create_block();
            let body = b.create_block();
            let advance = b.create_block();
            let unequal = b.create_block();
            let equal = b.create_block();
            let done = b.create_block();
            b.append_block_param(header, types::I64);
            b.append_block_param(done, types::I64);
            b.ins().brif(same, header, &[zero.into()], unequal, &[]);
            b.switch_to_block(header);
            let index = b.block_params(header)[0];
            let more = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
            b.ins().brif(more, body, &[], equal, &[]);
            b.switch_to_block(body);
            let a = b.ins().iadd(left, index);
            let c = b.ins().iadd(right, index);
            let a = b.ins().load(types::I8, MemFlags::new(), a, 0);
            let c = b.ins().load(types::I8, MemFlags::new(), c, 0);
            let same = b.ins().icmp(IntCC::Equal, a, c);
            b.ins().brif(same, advance, &[], unequal, &[]);
            b.switch_to_block(advance);
            let next = b.ins().iadd_imm(index, 1);
            b.ins().jump(header, &[next.into()]);
            b.switch_to_block(unequal);
            let word = b.ins().iconst(types::I64, i64::from(negate));
            b.ins().jump(done, &[word.into()]);
            b.switch_to_block(equal);
            let word = b.ins().iconst(types::I64, i64::from(!negate));
            b.ins().jump(done, &[word.into()]);
            b.switch_to_block(done);
            put(
                b,
                locals,
                registers,
                output.get() as usize,
                b.block_params(done)[0],
                MemFlags::new(),
            )?;
        }
    }
    if let Some(words) = view {
        put_view(options, b, locals, registers, layout.slot(output), words[0])?;
        put_view(
            options,
            b,
            locals,
            registers,
            layout.slot(output) + 1,
            words[1],
        )?;
    }
    Ok(())
}
