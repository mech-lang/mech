//! Pause only at the original header, before admitting another Tail traversal.
//! All carrier tests and state transitions are emitted instructions, not Rust.
use super::super::{
    NativeCompileError,
    abi::STATUS_ABI,
    compiler::{guard, offset},
    program_abi, program_borrowed_layout,
    program_hooks::Hooks,
    program_prefix_layout::Region,
    program_tail_layout,
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

pub(in crate::native_artifact) fn resume(
    b: &mut FunctionBuilder,
    locals: Value,
    region: &Region,
    borrowed: &program_borrowed_layout::Layout,
    closed: &program_tail_layout::Layout,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let site = region.tail.header() as usize;
    super::borrowed_codegen::resume(
        b,
        locals,
        &borrowed.regions[region.borrowed],
        site,
        hooks,
        error,
    )?;
    super::tail_codegen::resume(b, locals, closed, site, hooks, error)
}

pub(in crate::native_artifact) fn gate(
    b: &mut FunctionBuilder,
    frame: Value,
    locals: Value,
    phase: Value,
    region: &Region,
    borrowed: &program_borrowed_layout::Layout,
    closed: &program_tail_layout::Layout,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let prefix = b.create_block();
    let evaluate = b.create_block();
    let proceed = b.create_block();
    let prefix_phase = b
        .ins()
        .icmp_imm(IntCC::Equal, phase, program_abi::PHASE_PREFIX as i64);
    b.ins().brif(prefix_phase, prefix, &[], proceed, &[]);
    b.switch_to_block(prefix);
    let eof = b.ins().load(
        types::I64,
        MemFlags::new(),
        frame,
        core::mem::offset_of!(program_abi::Frame, input_eof) as i32,
    );
    let finished = b.ins().icmp_imm(IntCC::Equal, eof, 1);
    b.ins().brif(finished, proceed, &[], evaluate, &[]);
    b.switch_to_block(evaluate);
    let available = b.ins().load(
        types::I64,
        MemFlags::new(),
        frame,
        core::mem::offset_of!(program_abi::Frame, available_lines) as i32,
    );
    let ordinal = b
        .ins()
        .load(types::I64, MemFlags::new(), locals, offset(region.carrier)?);
    let mut known = b.ins().iconst(types::I8, 0);
    let mut pending = b.ins().iconst(types::I8, 0);
    for state in region.frontier.states() {
        let matches = b
            .ins()
            .icmp_imm(IntCC::Equal, ordinal, i64::from(state.ordinal()));
        known = b.ins().bor(known, matches);
        let waiting = if let Some(field) = state.index_field() {
            let index = b.ins().load(
                types::I64,
                MemFlags::new(),
                locals,
                offset(region.carrier + field)?,
            );
            let absent = b.ins().icmp(IntCC::UnsignedGreaterThan, index, available);
            b.ins().band(matches, absent)
        } else {
            // Even an already returned range must await the SAME capture's
            // EOF/error validation before finish/join/output can be reached.
            matches
        };
        pending = b.ins().bor(pending, waiting);
    }
    guard(b, known, error, STATUS_ABI);
    let pause = b.create_block();
    b.ins().brif(pending, pause, &[], proceed, &[]);
    b.switch_to_block(pause);
    let site = region.tail.header() as usize;
    super::tail_codegen::pause(b, locals, closed, site, hooks, error)?;
    let layout = &borrowed.regions[region.borrowed];
    let zero = b.ins().iconst(types::I64, 0);
    // Clear all raw pair lanes, including derived Strings from the last
    // traversal, BEFORE the helper drops their actual immutable owner loans.
    for word in layout.ready..layout.end {
        b.ins().store(MemFlags::new(), zero, locals, offset(word)?);
    }
    hooks.call(b, site, program_abi::PREFIX_PAUSE, [zero; 3], error);
    let status = b
        .ins()
        .iconst(types::I64, program_abi::STATUS_INPUT_PENDING as i64);
    b.ins().return_(&[status]);
    b.switch_to_block(proceed);
    Ok(())
}
