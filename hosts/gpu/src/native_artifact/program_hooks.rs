//! Emission of explicit leaf/transport hooks, never a graph evaluator.
use super::NativeCompileError;
use super::compiler::guard;
#[cfg(feature = "canonical-native-program")]
use cranelift_codegen::ir::MemFlags;
use cranelift_codegen::ir::{Block, InstBuilder, SigRef, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;

pub(super) struct Hooks {
    signature: SigRef,
    function: Value,
    context: Value,
    pub result: Value,
    #[cfg(feature = "canonical-native-program")]
    pub(super) arguments: Value,
    #[cfg(feature = "canonical-native-program")]
    argument_capacity: Value,
    #[cfg(feature = "canonical-native-program")]
    bound_shim: Option<(cranelift_codegen::ir::FuncRef, Value, Value, Value)>,
    #[cfg(feature = "canonical-native-program")]
    bound_shim_calls: std::cell::Cell<usize>,
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests {
    use super::*;
    use cranelift_codegen::ir::AbiParam;
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_module::Linkage;

    fn answer(marker: u64, values: [u64; 5]) -> u64 {
        values
            .into_iter()
            .zip([3, 5, 7, 11, 13])
            .fold(marker, |value, (input, weight)| {
                value.wrapping_add(input.wrapping_mul(weight))
            })
    }
    unsafe extern "C" fn record(
        raw: u64,
        site: u64,
        operation: u64,
        first: u64,
        second: u64,
        third: u64,
        result: *mut u64,
    ) -> u64 {
        // SAFETY: each context is an exclusive three-word slice during the
        // synchronous call; the caller supplies a separate live result word.
        let words = unsafe { &mut *(raw as *mut [u64; 3]) };
        words[0] += 1;
        unsafe {
            *result = answer(words[1], [site, operation, first, second, third]);
        }
        words[2]
    }
    unsafe extern "C" fn record_other(
        raw: u64,
        site: u64,
        operation: u64,
        first: u64,
        second: u64,
        third: u64,
        result: *mut u64,
    ) -> u64 {
        // SAFETY: same context and result contract as record().
        let status = unsafe { record(raw, site, operation, first, second, third, result) };
        unsafe {
            *result = (*result).wrapping_add(1000);
        }
        status
    }

    #[test]
    fn preserving_calls_hoist_only_exact_bindings_and_keep_changed_helpers_contexts_and_errors() {
        if !cfg!(target_arch = "aarch64") {
            return;
        }
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[
                ("opt_level", "speed"),
                ("preserve_frame_pointers", "true"),
                ("unwind_info", "true"),
            ],
            cranelift_module::default_libcall_names(),
        )
        .unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let mut report = super::super::iteration_compiler::CodegenReport::default();
        let controls = crate::native_artifact::compilation::CompilationControls::new(
            crate::native_artifact::compilation::OptimizationOptions::default(),
        );
        let shim = super::super::program_bound_shim::define(
            &controls,
            &mut module,
            "binding_test",
            super::super::NativeCompileLimits::default(),
            &mut report,
        )
        .unwrap()
        .unwrap();
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(types::I64));
        signature.returns.push(AbiParam::new(types::I64));
        let caller = module
            .declare_function("binding_caller", Linkage::Export, &signature)
            .unwrap();
        let mut context = module.make_context();
        context.func.signature = signature;
        let mut fb = FunctionBuilderContext::new();
        let inputs = [
            [2u64, 3, 5, 7, 11],
            [13, 17, 19, 23, 29],
            [31, 37, 41, 43, 47],
            [53, 59, 61, 67, 71],
        ];
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let pointer = b.block_params(entry)[0];
            let other_context = b.ins().iadd_imm(pointer, 24);
            let result = b.ins().iadd_imm(pointer, 48);
            let function = b
                .ins()
                .iconst(types::I64, record as *const () as usize as i64);
            let other_function = b
                .ins()
                .iconst(types::I64, record_other as *const () as usize as i64);
            let mut native = module.make_signature();
            native.params = vec![AbiParam::new(types::I64); 7];
            native.returns.push(AbiParam::new(types::I64));
            let zero = b.ins().iconst(types::I64, 0);
            let hooks = Hooks {
                signature: b.import_signature(native),
                function,
                context: pointer,
                result,
                arguments: pointer,
                argument_capacity: zero,
                bound_shim: None,
                bound_shim_calls: std::cell::Cell::new(0),
            }
            .with_bound_shim(&mut b, &mut module, shim.function, function, pointer);
            let error = b.create_block();
            b.append_block_param(error, types::I64);
            let mut total = zero;
            for (index, input) in inputs.iter().enumerate() {
                let arguments =
                    [input[2], input[3], input[4]].map(|v| b.ins().iconst(types::I64, v as i64));
                hooks.bound_call(
                    &mut b,
                    if index == 2 { other_function } else { function },
                    if index == 3 { other_context } else { pointer },
                    input[0] as usize,
                    input[1],
                    arguments,
                    error,
                );
                let value = b.ins().load(types::I64, MemFlags::new(), result, 0);
                total = b.ins().iadd(total, value);
            }
            assert_eq!(hooks.bound_shim_calls(), 2);
            b.ins().return_(&[total]);
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            b.ins().return_(&[status]);
            b.seal_all_blocks();
            b.finalize();
        }
        module.define_function(caller, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: this module owns the matching pointer -> u64 definition
        // throughout each synchronous call and every pointed-to word is live.
        let call: unsafe extern "C" fn(*mut u64) -> u64 =
            unsafe { std::mem::transmute(module.get_finalized_function(caller)) };
        for (first_status, second_status) in [(0, 0), (0, 7), (0, 0xffff_ffff_0000_0000), (7, 0)] {
            let mut words = [0, 123, first_status, 0, 456, second_status, u64::MAX];
            let expected = if first_status != 0 || second_status != 0 {
                super::super::abi::STATUS_ADMISSION
            } else {
                inputs
                    .iter()
                    .enumerate()
                    .map(|(index, input)| {
                        answer(if index == 3 { 456 } else { 123 }, *input)
                            + if index == 2 { 1000 } else { 0 }
                    })
                    .sum()
            };
            assert_eq!(unsafe { call(words.as_mut_ptr()) }, expected);
            assert_eq!(words[0], if first_status != 0 { 1 } else { 3 });
            assert_eq!(words[3], if first_status != 0 { 0 } else { 1 });
            assert_eq!(
                [words[1], words[2], words[4], words[5]],
                [123, first_status, 456, second_status]
            );
        }
    }
}
impl Hooks {
    #[cfg(feature = "canonical-native-program")]
    pub(super) fn with_bound_shim<M: Module>(
        mut self,
        b: &mut FunctionBuilder<'_>,
        module: &mut M,
        function: cranelift_module::FuncId,
        installed: Value,
        context: Value,
    ) -> Self {
        use cranelift_codegen::ir::{StackSlotData, StackSlotKind};
        let slot = b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            super::program_bound_shim::CALL_BYTES,
            3,
        ));
        let scratch = b.ins().stack_addr(types::I64, slot, 0);
        // These exact SSA values already remain fixed for this invocation.
        // Their storage belongs to this synchronous call frame and the bridge
        // only writes its separate status word. Keep them out of the loop's
        // live register set; each source call still supplies its own operands.
        for (index, value) in [(0, installed), (1, context), (7, self.result)] {
            b.ins().store(MemFlags::new(), value, scratch, index * 8);
        }
        self.bound_shim = Some((
            module.declare_func_in_func(function, b.func),
            scratch,
            installed,
            context,
        ));
        self
    }
    #[cfg(feature = "canonical-native-program")]
    pub(super) fn bound_shim_calls(&self) -> usize {
        self.bound_shim_calls.get()
    }
    #[cfg(feature = "canonical-native-program")]
    pub(super) fn bound_shim_work(&self) -> usize {
        self.bound_shim_calls() * super::program_bound_shim::TEMPLATE_BOUND
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn require_argument_words(&self, b: &mut FunctionBuilder<'_>, count: usize, error: Block) {
        let enough = b.ins().icmp_imm(
            IntCC::UnsignedGreaterThanOrEqual,
            self.argument_capacity,
            count as i64,
        );
        guard(b, enough, error, super::abi::STATUS_ABI);
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn write_entry_words(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &[Value],
        error: Block,
    ) -> Result<(), NativeCompileError> {
        let enough = b.ins().icmp_imm(
            IntCC::UnsignedGreaterThanOrEqual,
            self.argument_capacity,
            words.len() as i64,
        );
        guard(b, enough, error, super::abi::STATUS_ABI);
        for (index, &word) in words.iter().enumerate() {
            b.ins().store(
                MemFlags::new(),
                word,
                self.arguments,
                super::compiler::offset(index)?,
            );
        }
        Ok(())
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn bound_call(
        &self,
        b: &mut FunctionBuilder<'_>,
        function: Value,
        context: Value,
        site: usize,
        operation: u64,
        args: [Value; 3],
        error: Block,
    ) {
        let site = b.ins().iconst(types::I64, site as i64);
        let operation = b.ins().iconst(types::I64, operation as i64);
        // A distinct function/context value has no binding proof. Retain the
        // ordinary native call rather than using an earlier helper's words.
        let binding = self
            .bound_shim
            .filter(|&(_, _, installed, owner)| installed == function && owner == context);
        let status = if let Some((shim, scratch, _, _)) = binding {
            for (index, value) in [site, operation, args[0], args[1], args[2]]
                .into_iter()
                .enumerate()
            {
                b.ins()
                    .store(MemFlags::new(), value, scratch, ((index + 2) * 8) as i32);
            }
            b.ins().call(shim, &[scratch]);
            self.bound_shim_calls.set(self.bound_shim_calls.get() + 1);

            b.ins().load(
                types::I64,
                MemFlags::new(),
                scratch,
                super::program_bound_shim::STATUS_OFFSET,
            )
        } else {
            let call = b.ins().call_indirect(
                self.signature,
                function,
                &[
                    context,
                    site,
                    operation,
                    args[0],
                    args[1],
                    args[2],
                    self.result,
                ],
            );
            b.inst_results(call)[0]
        };
        let ok = b.ins().icmp_imm(IntCC::Equal, status, 0);
        guard(b, ok, error, super::abi::STATUS_ADMISSION);
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn joined_string_map(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        function: cranelift_codegen::ir::FuncRef,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        use super::compiler::offset;
        use super::program_abi::{JOIN_MAP_FINISH, JOIN_MAP_PREPARE};
        self.require_argument_words(b, 5, error);
        let zero = b.ins().iconst(types::I64, 0);
        let header = b.create_block();
        let native = b.create_block();
        let done = b.create_block();
        let ready = b.ins().load(types::I64, MemFlags::new(), self.result, 0);
        let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, ready, 1);
        guard(b, valid, error, super::abi::STATUS_ABI);
        b.ins().brif(ready, header, &[], done, &[]);
        b.switch_to_block(header);
        self.call(b, site, JOIN_MAP_PREPARE, [zero; 3], error);
        let more = b.ins().load(types::I64, MemFlags::new(), self.result, 0);
        let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, more, 1);
        guard(b, valid, error, super::abi::STATUS_ABI);
        b.ins().brif(more, native, &[], done, &[]);
        b.switch_to_block(native);
        let mut words = Vec::new();
        for index in 0..5 {
            words.push(
                b.ins()
                    .load(types::I64, MemFlags::new(), self.arguments, offset(index)?),
            );
        }
        let call = b.ins().call(function, &words);
        let processed = b.inst_results(call)[0];
        self.call(b, site, JOIN_MAP_FINISH, [processed, zero, zero], error);
        b.ins().jump(header, &[]);
        b.switch_to_block(done);
        Ok(())
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn string_map_region(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        region: &mech_engine::native_artifact::NativeStringMapRegion,
        function: cranelift_codegen::ir::FuncRef,
        locals: Value,
        counter_slot: usize,
        exit: Block,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        use super::compiler::offset;
        use super::program_abi::{STRING_MAP_FINISH, STRING_MAP_PREPARE};
        use cranelift_codegen::ir::MemFlags;
        let enough = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, self.argument_capacity, 5);
        guard(b, enough, error, super::abi::STATUS_ABI);
        let remaining = b
            .ins()
            .load(types::I64, MemFlags::new(), locals, offset(counter_slot)?);
        let collection = b.ins().iconst(types::I64, region.collection() as i64);
        let zero = b.ins().iconst(types::I64, 0);
        self.call(
            b,
            site,
            STRING_MAP_PREPARE,
            [remaining, collection, zero],
            error,
        );
        let ready = b.ins().load(types::I64, MemFlags::new(), self.result, 0);
        let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, ready, 2);
        guard(b, valid, error, super::abi::STATUS_ABI);
        let complete = b.ins().icmp_imm(IntCC::Equal, ready, 2);
        let not_complete = b.create_block();
        b.ins().brif(complete, exit, &[], not_complete, &[]);
        b.switch_to_block(not_complete);
        let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
        let native = b.create_block();
        let ordinary = b.create_block();
        b.ins().brif(ready, native, &[], ordinary, &[]);
        b.switch_to_block(native);
        let words = (0..5)
            .map(|index| {
                Ok(b.ins()
                    .load(types::I64, MemFlags::new(), self.arguments, offset(index)?))
            })
            .collect::<Result<Vec<_>, NativeCompileError>>()?;
        let call = b.ins().call(function, &words);
        let processed = b.inst_results(call)[0];
        self.call(
            b,
            site,
            STRING_MAP_FINISH,
            [processed, collection, zero],
            error,
        );
        b.ins().jump(exit, &[]);
        b.switch_to_block(ordinary);
        Ok(())
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn window_region(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        region: &mech_engine::native_artifact::NativeWindowRegion,
        function: cranelift_codegen::ir::FuncRef,
        locals: Value,
        counter_slot: usize,
        exit: Block,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        use super::{
            compiler::{guard, offset},
            program_abi::{WINDOW_FINISH, WINDOW_PREPARE},
        };
        use cranelift_codegen::ir::MemFlags;
        let required =
            b.ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, self.argument_capacity, 5);
        guard(b, required, error, super::abi::STATUS_ABI);
        let width = super::control_compiler::load(b, locals, region.width())?;
        let length = super::control_compiler::load(b, locals, region.length())?;
        // Only attempt representation binding at the first header. A declined
        // Unicode/CRLF variant must not rescan or rebind on each iteration.
        let ordinary = b.create_block();
        let attempt = b.create_block();
        let remaining = b
            .ins()
            .load(types::I64, MemFlags::new(), locals, offset(counter_slot)?);
        let initial = b.ins().iadd_imm(length, 1);
        let first = b.ins().icmp(IntCC::Equal, remaining, initial);
        b.ins().brif(first, attempt, &[], ordinary, &[]);
        b.switch_to_block(attempt);
        let collection = b.ins().iconst(types::I64, region.collection() as i64);
        self.call(b, site, WINDOW_PREPARE, [width, length, collection], error);
        let ready = b.ins().load(types::I64, MemFlags::new(), self.result, 0);
        let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
        let native = b.create_block();
        b.ins().brif(ready, native, &[], ordinary, &[]);
        b.switch_to_block(native);
        let words = (0..5)
            .map(|index| {
                Ok(b.ins()
                    .load(types::I64, MemFlags::new(), self.arguments, offset(index)?))
            })
            .collect::<Result<Vec<_>, NativeCompileError>>()?;
        // Direct call to code emitted from THIS plan, not a runtime executor
        // or source helper. The entire loop contains no callback/admission.
        let call = b.ins().call(function, &words);
        let count = b.inst_results(call)[0];
        let zero = b.ins().iconst(types::I64, 0);
        self.call(b, site, WINDOW_FINISH, [count, collection, zero], error);
        b.ins().jump(exit, &[]);
        b.switch_to_block(ordinary);
        Ok(())
    }
    pub fn load<M: Module>(
        mixed: bool,
        b: &mut FunctionBuilder<'_>,
        module: &M,
        frame: Value,
        context: Value,
        error: Block,
    ) -> Result<Option<Self>, NativeCompileError> {
        if !mixed {
            return Ok(None);
        }
        #[cfg(not(feature = "canonical-native-program"))]
        {
            let _ = (b, module, frame, context, error);
            Err(super::compiler::backend("mixed ABI is not enabled"))
        }
        #[cfg(feature = "canonical-native-program")]
        {
            let function = super::compiler::frame_load(
                b,
                frame,
                core::mem::offset_of!(super::program_abi::Frame, helper),
            );
            let result = super::compiler::frame_load(
                b,
                frame,
                core::mem::offset_of!(super::program_abi::Frame, result),
            );
            let arguments = super::compiler::frame_load(
                b,
                frame,
                core::mem::offset_of!(super::program_abi::Frame, arguments),
            );
            let argument_capacity = super::compiler::frame_load(
                b,
                frame,
                core::mem::offset_of!(super::program_abi::Frame, argument_capacity),
            );
            for pointer in [function, result, arguments] {
                let valid = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
                guard(b, valid, error, super::abi::STATUS_ABI);
            }
            let mut signature = module.make_signature();
            signature.params = vec![cranelift_codegen::ir::AbiParam::new(types::I64); 7];
            signature
                .returns
                .push(cranelift_codegen::ir::AbiParam::new(types::I64));
            Ok(Some(Self {
                signature: b.import_signature(signature),
                function,
                context,
                result,
                arguments,
                argument_capacity,
                bound_shim: None,
                bound_shim_calls: std::cell::Cell::new(0),
            }))
        }
    }
    pub fn library(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        operands: &[mech_engine::native_artifact::NativeMatchValue],
        locals: Value,
        is_scalar: impl Fn(mech_engine::native_artifact::NativeMatchValue) -> bool,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        self.library_operation(b, site, 0, operands, locals, is_scalar, error)
    }
    pub fn library_operation(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        operation: u64,
        operands: &[mech_engine::native_artifact::NativeMatchValue],
        locals: Value,
        is_scalar: impl Fn(mech_engine::native_artifact::NativeMatchValue) -> bool,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        #[cfg(feature = "canonical-native-program")]
        {
            let count = b.ins().iconst(types::I64, operands.len() as i64);
            let valid = b.ins().icmp(
                IntCC::UnsignedGreaterThanOrEqual,
                self.argument_capacity,
                count,
            );
            guard(b, valid, error, super::abi::STATUS_ABI);
            for (index, &operand) in operands.iter().enumerate() {
                let value = if is_scalar(operand) {
                    super::control_compiler::load(b, locals, operand)?
                } else {
                    // Managed operands are fetched from their exact generation
                    // in the canonical Value arena, never from native pointers.
                    b.ins().iconst(types::I64, 0)
                };
                b.ins().store(
                    cranelift_codegen::ir::MemFlags::new(),
                    value,
                    self.arguments,
                    super::compiler::offset(index)?,
                );
            }
            let zero = b.ins().iconst(types::I64, 0);
            self.call(b, site, operation, [self.arguments, count, zero], error);
            Ok(())
        }
        #[cfg(not(feature = "canonical-native-program"))]
        {
            let _ = (b, site, operation, operands, locals, is_scalar, error);
            Err(super::compiler::backend(
                "library operand ABI is not enabled",
            ))
        }
    }
    pub fn call(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        operation: u64,
        args: [Value; 3],
        error: Block,
    ) {
        let site = b.ins().iconst(types::I64, site as i64);
        self.call_at(b, site, operation, args, error);
    }
    pub fn call_at(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: Value,
        operation: u64,
        args: [Value; 3],
        error: Block,
    ) {
        let operation = b.ins().iconst(types::I64, operation as i64);
        let call = b.ins().call_indirect(
            self.signature,
            self.function,
            &[
                self.context,
                site,
                operation,
                args[0],
                args[1],
                args[2],
                self.result,
            ],
        );
        let status = b.inst_results(call)[0];
        let valid = b
            .ins()
            .icmp_imm(IntCC::Equal, status, super::abi::STATUS_OK as i64);
        guard(b, valid, error, super::abi::STATUS_ADMISSION);
    }
    pub fn collection(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        args: [Value; 2],
        error: Block,
    ) -> Result<(), NativeCompileError> {
        #[cfg(feature = "canonical-native-program")]
        {
            let zero = b.ins().iconst(types::I64, 0);
            self.call(
                b,
                site,
                super::program_abi::COLLECTION,
                [args[0], args[1], zero],
                error,
            );
            Ok(())
        }
        #[cfg(not(feature = "canonical-native-program"))]
        {
            let _ = (b, site, args, error);
            Err(super::compiler::backend("collection ABI is not enabled"))
        }
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn output(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        ordinal: usize,
        value: Value,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        let operation = super::program_abi::OUTPUT_BASE
            .checked_add(ordinal as u64)
            .ok_or_else(|| super::compiler::backend("output helper ordinal overflow"))?;
        let zero = b.ins().iconst(types::I64, 0);
        self.call(b, site, operation, [value, zero, zero], error);
        Ok(())
    }
}
