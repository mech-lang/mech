use super::{AArch64Backend, inst::LabelUse, settings as arm_settings};
use crate::Context;
use crate::cursor::{Cursor, FuncCursor};
use crate::ir::{
    AbiParam, DebugTag, ExtFuncData, ExternalName, Function, InstBuilder, Signature, StackSlotData,
    StackSlotKind, UserExternalName, UserFuncName, ValueLabel, ValueLabelAssignments,
    ValueLabelStart, types::I64,
};
use crate::isa::CallConv;
use crate::machinst::{CompiledCode, EmitResult, MachInstLabelUse};
use crate::settings::{self, Configurable};
use alloc::vec::Vec;
use cranelift_control::ControlPlane;

const APPLE: &str = "aarch64-apple-darwin";
const RET: u32 = 0xd65f03c0;
const RETAB: u32 = 0xd65f0fff;
// ldp x29, x30, [sp], #16
const RESTORE_FRAME: u32 = 0xa8c17bfd;

fn backend(target: &str, frame: bool, authenticate: bool, cfg_info: bool) -> AArch64Backend {
    let mut flags = settings::builder();
    flags
        .set(
            "preserve_frame_pointers",
            if frame { "true" } else { "false" },
        )
        .unwrap();
    flags
        .set(
            "machine_code_cfg_info",
            if cfg_info { "true" } else { "false" },
        )
        .unwrap();
    flags.enable("unwind_info").unwrap();
    let flags = settings::Flags::new(flags);
    let mut arm_flags = arm_settings::builder();
    if authenticate {
        arm_flags.enable("has_pauth").unwrap();
        arm_flags.enable("sign_return_address").unwrap();
        arm_flags.enable("sign_return_address_with_bkey").unwrap();
    }
    let arm_flags = arm_settings::Flags::new(&flags, &arm_flags);
    AArch64Backend::new_with_flags(target.parse().unwrap(), flags, arm_flags)
}

fn function(large_frame: bool) -> Function {
    let mut signature = Signature::new(CallConv::AppleAarch64);
    signature.params = vec![AbiParam::new(I64); 2];
    signature.returns = vec![AbiParam::new(I64); 2];
    let mut function = Function::with_name_signature(UserFuncName::user(0, 0), signature);
    let mut block = || {
        let block = function.dfg.make_block();
        function.layout.append_block(block);
        block
    };
    let entry = block();
    let success = block();
    let dispatch = block();
    let first_error = block();
    let second_error = block();
    function.layout.set_cold(first_error);
    function.layout.set_cold(second_error);
    let test = function.dfg.append_block_param(entry, I64);
    let which_error = function.dfg.append_block_param(entry, I64);

    let mut cursor = FuncCursor::new(&mut function);
    cursor.goto_bottom(entry);
    let test = if large_frame {
        let slot = cursor.func.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            49_296,
            4,
        ));
        let signature = cursor
            .func
            .import_signature(Signature::new(CallConv::AppleAarch64));
        let name = cursor
            .func
            .declare_imported_user_function(UserExternalName {
                namespace: 0,
                index: 1,
            });
        let callee = cursor.func.import_function(ExtFuncData {
            name: ExternalName::user(name),
            signature,
            colocated: true,
            patchable: false,
        });
        cursor.ins().stack_store(test, slot, 0);
        // which_error remains live across this call.
        cursor.ins().call(callee, &[]);
        cursor.ins().stack_load(I64, slot, 0)
    } else {
        test
    };
    cursor.ins().brif(test, success, &[], dispatch, &[]);
    cursor.goto_bottom(success);
    let zero = cursor.ins().iconst(I64, 0);
    cursor.ins().return_(&[zero, zero]);
    cursor.goto_bottom(dispatch);
    cursor
        .ins()
        .brif(which_error, first_error, &[], second_error, &[]);
    for (block, status, origin) in [(first_error, 1, 17), (second_error, 2, 34)] {
        cursor.goto_bottom(block);
        let status = cursor.ins().iconst(I64, status);
        let origin = cursor.ins().iconst(I64, origin);
        cursor.ins().return_(&[status, origin]);
    }
    function
}

fn compile(backend: &AArch64Backend, mut function: Function, share: bool) -> CompiledCode {
    function.share_cold_epilogues = share;
    let mut context = Context::for_function(function);
    context
        .compile(backend, &mut ControlPlane::default())
        .unwrap();
    context.take_compiled_code().unwrap()
}

fn words(bytes: &[u8]) -> impl Iterator<Item = u32> + '_ {
    bytes
        .chunks_exact(4)
        .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
}

fn return_indices(bytes: &[u8], return_word: u32) -> Vec<usize> {
    words(bytes)
        .enumerate()
        .filter_map(|(index, word)| (word == return_word).then_some(index))
        .collect()
}

#[test]
fn sharing_keeps_the_success_return_and_prologue_unwind() {
    for (authenticate, ret) in [(false, RET), (true, RETAB)] {
        let backend = backend(APPLE, true, authenticate, false);
        let control = compile(&backend, function(false), false);
        let candidate = compile(&backend, function(false), true);
        assert_eq!(control.buffer.unwind_info, candidate.buffer.unwind_info);

        let control = control.code_buffer();
        let candidate = candidate.code_buffer();
        let old_returns = return_indices(control, ret);
        let new_returns = return_indices(candidate, ret);
        assert_eq!(old_returns.len(), 3);
        assert_eq!(new_returns.len(), 2);
        assert_eq!(old_returns[0], new_returns[0]);
        let tail = new_returns[1] - 1;
        assert_eq!(words(candidate).nth(tail), Some(RESTORE_FRAME));
        assert!(words(candidate).enumerate().any(|(index, word)| {
            // B immediate with a signed 26-bit displacement in words.
            word >> 26 == 0b000101 && index as i32 + ((word << 6) as i32 >> 6) == tail as i32
        }));
        assert_eq!(control.len() - candidate.len(), 4);
    }
}

#[test]
fn out_of_range_returns_emit_another_epilogue() {
    let backend = backend(APPLE, true, false, false);
    let emit = |range| -> EmitResult {
        let mut context = Context::for_function(function(false));
        let mut control = ControlPlane::default();
        context.optimize(&backend, &mut control).unwrap();
        let (vcode, allocated) = backend
            .compile_vcode(&context.func, &context.domtree, &mut control)
            .unwrap();
        vcode.emit_with_cold_epilogue_sharing(
            &allocated,
            false,
            &backend.flags,
            &mut control,
            range,
        )
    };
    let control = emit(None);
    let returns = return_indices(control.buffer.data(), RET);
    assert_eq!(returns.len(), 3);
    let distance = u32::try_from(4 * (returns[2] - returns[1])).unwrap();
    let too_far = emit(Some(distance - 1));
    let in_range = emit(Some(distance));
    assert_eq!(too_far.buffer.data(), control.buffer.data());
    assert_eq!(return_indices(in_range.buffer.data(), RET).len(), 2);
    assert_eq!(LabelUse::Branch26.max_neg_range(), 128 * 1024 * 1024);
}

#[test]
fn single_instruction_epilogue_keeps_its_return() {
    let backend = backend(APPLE, false, false, false);
    let control = compile(&backend, function(false), false);
    let candidate = compile(&backend, function(false), true);
    assert_eq!(return_indices(control.code_buffer(), RET).len(), 3);
    assert!(!words(control.code_buffer()).any(|word| word == RESTORE_FRAME));
    assert_eq!(control.code_buffer(), candidate.code_buffer());
}

#[test]
fn other_target_or_calling_convention_keeps_its_returns() {
    for (target, convention) in [
        ("aarch64-unknown-linux-gnu", CallConv::AppleAarch64),
        (APPLE, CallConv::SystemV),
    ] {
        let backend = backend(target, true, false, false);
        let mut input = function(false);
        input.signature.call_conv = convention;
        let control = compile(&backend, input.clone(), false);
        let candidate = compile(&backend, input, true);
        assert_eq!(control.code_buffer(), candidate.code_buffer());
    }
}

#[test]
fn machine_cfg_info_keeps_the_original_return_blocks() {
    let backend = backend(APPLE, true, false, true);
    let control = compile(&backend, function(false), false);
    let candidate = compile(&backend, function(false), true);
    assert_eq!(control.code_buffer(), candidate.code_buffer());
    assert_eq!(control.bb_starts, candidate.bb_starts);
    assert_eq!(control.bb_edges, candidate.bb_edges);
}

#[test]
fn debug_tags_and_value_labels_keep_the_original_return_blocks() {
    let backend = backend(APPLE, true, false, false);
    let mut tagged = function(false);
    let entry = tagged.layout.entry_block().unwrap();
    let first = tagged.layout.first_inst(entry).unwrap();
    let mut cursor = FuncCursor::new(&mut tagged);
    cursor.goto_inst(first);
    let point = cursor.ins().sequence_point();
    tagged.debug_tags.set(point, [DebugTag::User(1)]);
    let tagged_control = compile(&backend, tagged.clone(), false);
    let tagged_candidate = compile(&backend, tagged, true);
    assert!(tagged_candidate.buffer.debug_tags().next().is_some());
    assert_eq!(tagged_control.code_buffer(), tagged_candidate.code_buffer());

    let mut labelled = function(false);
    let entry = labelled.layout.entry_block().unwrap();
    let labelled_arg = labelled.dfg.block_params(entry)[0];
    labelled.collect_debug_info();
    labelled.dfg.values_labels.as_mut().unwrap().insert(
        labelled_arg,
        ValueLabelAssignments::Starts(vec![ValueLabelStart {
            from: Default::default(),
            label: ValueLabel::from_u32(1),
        }]),
    );
    let labelled_control = compile(&backend, labelled.clone(), false);
    let labelled_candidate = compile(&backend, labelled, true);
    assert!(!labelled_candidate.value_labels_ranges.is_empty());
    assert_eq!(
        labelled_control.code_buffer(),
        labelled_candidate.code_buffer()
    );
}

#[test]
fn large_frame_keeps_saved_register_restores_and_authenticated_return() {
    let backend = backend(APPLE, true, true, false);
    let control = compile(&backend, function(true), false);
    let candidate = compile(&backend, function(true), true);
    assert_eq!(return_indices(control.code_buffer(), RETAB).len(), 3);
    assert_eq!(return_indices(candidate.code_buffer(), RETAB).len(), 2);
    assert_eq!(control.buffer.unwind_info, candidate.buffer.unwind_info);
    assert!(candidate.buffer.unwind_info.iter().any(|(_, op)| {
        matches!(op, crate::isa::unwind::UnwindInst::StackAlloc { size } if *size >= 49_296)
    }));
    assert!(
        candidate
            .buffer
            .unwind_info
            .iter()
            .any(|(_, op)| { matches!(op, crate::isa::unwind::UnwindInst::SaveReg { .. }) })
    );
    assert!(control.code_buffer().len() - candidate.code_buffer().len() >= 12);
}

#[test]
fn clearing_a_function_resets_the_sharing_hint() {
    let mut function = function(false);
    function.share_cold_epilogues = true;
    function.clear();
    assert!(!function.share_cold_epilogues);
}
