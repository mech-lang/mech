//! Proof-bound entries and ordered result frontiers for the original CFG.
use super::super::{
    NativeCompileError, compiler::offset, program_abi as abi, program_hooks::Hooks,
    program_parallel_layout::Layout,
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeProgramArtifactPlan};

pub(in crate::native_artifact) fn dispatch(
    b: &mut FunctionBuilder<'_>,
    phase: Value,
    frame: Value,
    layout: &Layout,
    blocks: &[Option<Block>],
    hooks: &Hooks,
    error: Block,
) {
    let ordinary = b.create_block();
    let worker = b.create_block();
    let active = b
        .ins()
        .icmp_imm(IntCC::Equal, phase, abi::PHASE_WORKER as i64);
    b.ins().brif(active, worker, &[], ordinary, &[]);
    b.switch_to_block(worker);
    let target = super::super::compiler::frame_load(
        b,
        frame,
        core::mem::offset_of!(abi::Frame, resume_site),
    );
    for (id, region) in layout.regions.iter().enumerate() {
        let next = b.create_block();
        let enter = b.create_block();
        let matched = b
            .ins()
            .icmp_imm(IntCC::Equal, target, i64::from(region.begin));
        b.ins().brif(matched, enter, &[], next, &[]);
        b.switch_to_block(enter);
        let id = b.ins().iconst(types::I64, id as i64);
        let index = super::super::compiler::frame_load(
            b,
            frame,
            core::mem::offset_of!(abi::Frame, available_lines),
        );
        let count = super::super::compiler::frame_load(
            b,
            frame,
            core::mem::offset_of!(abi::Frame, input_eof),
        );
        hooks.call(
            b,
            region.begin as usize,
            abi::PARALLEL_ENTER,
            [id, index, count],
            error,
        );
        b.ins().jump(
            blocks[region.begin as usize].expect("proved parallel begin"),
            &[],
        );
        b.switch_to_block(next);
    }
    let status = b
        .ins()
        .iconst(types::I64, super::super::abi::STATUS_ABI as i64);
    b.ins().jump(error, &[status.into()]);
    b.switch_to_block(ordinary);
}

pub(in crate::native_artifact) fn before_visit(
    b: &mut FunctionBuilder<'_>,
    plan: &NativeProgramArtifactPlan,
    layout: &Layout,
    site: usize,
    phase: Value,
    frame: Value,
    locals: Value,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let Some((id, region)) = layout
        .regions
        .iter()
        .enumerate()
        .find(|(_, r)| r.push as usize == site)
    else {
        return Ok(());
    };
    let ordinary = b.create_block();
    let yielded = b.create_block();
    let worker = b
        .ins()
        .icmp_imm(IntCC::Equal, phase, abi::PHASE_WORKER as i64);
    let target = super::super::compiler::frame_load(
        b,
        frame,
        core::mem::offset_of!(abi::Frame, resume_site),
    );
    let exact = b
        .ins()
        .icmp_imm(IntCC::Equal, target, i64::from(region.begin));
    let selected = b.ins().band(worker, exact);
    b.ins().brif(selected, yielded, &[], ordinary, &[]);
    b.switch_to_block(yielded);
    let zero = b.ins().iconst(types::I64, 0);
    let word = if plan.values()[region.output as usize]
        .scalar_kind()
        .is_some()
    {
        b.ins().load(
            types::I64,
            MemFlags::new(),
            locals,
            offset(region.output as usize)?,
        )
    } else {
        zero
    };
    let id = b.ins().iconst(types::I64, id as i64);
    hooks.call(b, site, abi::PARALLEL_YIELD, [word, id, zero], error);
    let status = b.ins().iconst(types::I64, abi::STATUS_WORKER_RESULT as i64);
    b.ins().return_(&[status]);
    b.switch_to_block(ordinary);
    Ok(())
}

pub(in crate::native_artifact) fn after_action(
    b: &mut FunctionBuilder<'_>,
    plan: &NativeProgramArtifactPlan,
    layout: &Layout,
    site: usize,
    phase: Value,
    frame: Value,
    entry: Value,
    locals: Value,
    blocks: &[Option<Block>],
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let Some((id, region)) = layout
        .regions
        .iter()
        .enumerate()
        .find(|(_, r)| r.initialize as usize == site || r.body as usize == site)
    else {
        return Ok(());
    };
    let ordinary = b.create_block();
    let parent = b.create_block();
    let worker = b
        .ins()
        .icmp_imm(IntCC::Equal, phase, abi::PHASE_WORKER as i64);
    b.ins().brif(worker, ordinary, &[], parent, &[]);
    b.switch_to_block(parent);
    let id_word = b.ins().iconst(types::I64, id as i64);
    let zero = b.ins().iconst(types::I64, 0);
    if region.initialize as usize == site {
        hooks.call(b, site, abi::PARALLEL_RUN, [entry, frame, id_word], error);
    } else {
        if !matches!(
            plan.blocks()[site].action(),
            Action::CollectionElement { .. }
        ) {
            return Err(super::super::compiler::backend(
                "parallel body lacks original element",
            ));
        }
        hooks.call(b, site, abi::PARALLEL_TAKE, [id_word, zero, zero], error);
        let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
        let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
        let publish = b.create_block();
        b.ins().brif(ready, publish, &[], ordinary, &[]);
        b.switch_to_block(publish);
        if plan.values()[region.output as usize]
            .scalar_kind()
            .is_some()
        {
            hooks.require_argument_words(b, 1, error);
            let word = b
                .ins()
                .load(types::I64, MemFlags::new(), hooks.arguments, 0);
            b.ins().store(
                MemFlags::new(),
                word,
                locals,
                offset(region.output as usize)?,
            );
        }
        b.ins().jump(
            blocks[region.push as usize].expect("proved parallel push"),
            &[],
        );
        b.switch_to_block(ordinary);
        return Ok(());
    }
    b.ins().jump(ordinary, &[]);
    b.switch_to_block(ordinary);
    Ok(())
}
