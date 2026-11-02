//! Emission of explicit leaf/transport hooks, never a graph evaluator.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::guard;
use crate::native_artifact::program::protocol::GlobalHelper;
#[cfg(feature = "canonical-native-program")]
use crate::native_artifact::program::protocol::OwnerOperation;
#[cfg(feature = "canonical-native-program")]
use cranelift_codegen::ir::MemFlags;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, SigRef, Value, types};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;

pub(in crate::native_artifact) struct Hooks {
    signature: SigRef,
    function: Value,
    context: Value,
    pub result: Value,
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) arguments: Value,
    #[cfg(feature = "canonical-native-program")]
    argument_capacity: Value,
    #[cfg(feature = "canonical-native-program")]
    bound_shim: Option<(cranelift_codegen::ir::FuncRef, Value, Value, Value)>,
    #[cfg(feature = "canonical-native-program")]
    bound_shim_calls: std::cell::Cell<usize>,
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests;
impl Hooks {
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn with_bound_shim<M: Module>(
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
            crate::native_artifact::program::codegen::bound_shim::CALL_BYTES,
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
    pub(in crate::native_artifact) fn bound_shim_calls(&self) -> usize {
        self.bound_shim_calls.get()
    }
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn bound_shim_work(&self) -> usize {
        self.bound_shim_calls()
            * crate::native_artifact::program::codegen::bound_shim::TEMPLATE_BOUND
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn require_argument_words(&self, b: &mut FunctionBuilder<'_>, count: usize, error: Block) {
        let enough = b.ins().icmp_imm(
            IntCC::UnsignedGreaterThanOrEqual,
            self.argument_capacity,
            count as i64,
        );
        guard(b, enough, error, crate::native_artifact::abi::STATUS_ABI);
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
        guard(b, enough, error, crate::native_artifact::abi::STATUS_ABI);
        for (index, &word) in words.iter().enumerate() {
            b.ins().store(
                MemFlags::new(),
                word,
                self.arguments,
                crate::native_artifact::compiler::offset(index)?,
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
        operation: OwnerOperation,
        args: [Value; 3],
        error: Block,
    ) {
        let site = b.ins().iconst(types::I64, site as i64);
        let operation = b.ins().iconst(types::I64, operation.raw() as i64);
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
                crate::native_artifact::program::codegen::bound_shim::STATUS_OFFSET,
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
        guard(b, ok, error, crate::native_artifact::abi::STATUS_ADMISSION);
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn joined_string_map(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        function: cranelift_codegen::ir::FuncRef,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        use crate::native_artifact::compiler::offset;
        self.require_argument_words(b, 5, error);
        let zero = b.ins().iconst(types::I64, 0);
        let header = b.create_block();
        let native = b.create_block();
        let done = b.create_block();
        let ready = b.ins().load(types::I64, MemFlags::new(), self.result, 0);
        let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, ready, 1);
        guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
        b.ins().brif(ready, header, &[], done, &[]);
        b.switch_to_block(header);
        self.call(b, site, GlobalHelper::JoinMapPrepare, [zero; 3], error);
        let more = b.ins().load(types::I64, MemFlags::new(), self.result, 0);
        let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, more, 1);
        guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
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
        self.call(
            b,
            site,
            GlobalHelper::JoinMapFinish,
            [processed, zero, zero],
            error,
        );
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
        use crate::native_artifact::compiler::offset;
        use cranelift_codegen::ir::MemFlags;
        let enough = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, self.argument_capacity, 5);
        guard(b, enough, error, crate::native_artifact::abi::STATUS_ABI);
        let remaining = b
            .ins()
            .load(types::I64, MemFlags::new(), locals, offset(counter_slot)?);
        let collection = b.ins().iconst(types::I64, region.collection() as i64);
        let zero = b.ins().iconst(types::I64, 0);
        self.call(
            b,
            site,
            GlobalHelper::StringMapPrepare,
            [remaining, collection, zero],
            error,
        );
        let ready = b.ins().load(types::I64, MemFlags::new(), self.result, 0);
        let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, ready, 2);
        guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
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
            GlobalHelper::StringMapFinish,
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
        use crate::native_artifact::compiler::{guard, offset};
        use cranelift_codegen::ir::MemFlags;
        let required =
            b.ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, self.argument_capacity, 5);
        guard(b, required, error, crate::native_artifact::abi::STATUS_ABI);
        let width = crate::native_artifact::control_compiler::load(b, locals, region.width())?;
        let length = crate::native_artifact::control_compiler::load(b, locals, region.length())?;
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
        self.call(
            b,
            site,
            GlobalHelper::WindowPrepare,
            [width, length, collection],
            error,
        );
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
        self.call(
            b,
            site,
            GlobalHelper::WindowFinish,
            [count, collection, zero],
            error,
        );
        b.ins().jump(exit, &[]);
        b.switch_to_block(ordinary);
        Ok(())
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn uniform_words(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        region: &mech_engine::native_artifact::NativeUniformWordCollection,
        locals: Value,
        counter_slot: usize,
        exit: Block,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        use crate::native_artifact::compiler::offset;
        let length = crate::native_artifact::control_compiler::load(b, locals, region.limit())?;
        let remaining = b
            .ins()
            .load(types::I64, MemFlags::new(), locals, offset(counter_slot)?);
        // Preparation already rejected an overflowing inclusive range. Attempt
        // once per fresh visit; a resource decline keeps the ordinary CFG.
        let initial = b.ins().iadd_imm(length, 1);
        let first = b.ins().icmp(IntCC::Equal, remaining, initial);
        let attempt = b.create_block();
        let ordinary = b.create_block();
        b.ins().brif(first, attempt, &[], ordinary, &[]);
        b.switch_to_block(attempt);
        let word = crate::native_artifact::control_compiler::load(b, locals, region.word())?;
        let zero = b.ins().iconst(types::I64, 0);
        self.call(
            b,
            site,
            GlobalHelper::Collection,
            [word, length, zero],
            error,
        );
        let ready = b.ins().load(types::I64, MemFlags::new(), self.result, 0);
        let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, ready, 1);
        guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
        let completed = b.create_block();
        b.ins().brif(ready, completed, &[], ordinary, &[]);
        b.switch_to_block(completed);
        b.ins()
            .store(MemFlags::new(), zero, locals, offset(counter_slot)?);
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
            Err(crate::native_artifact::compiler::backend(
                "mixed ABI is not enabled",
            ))
        }
        #[cfg(feature = "canonical-native-program")]
        {
            let function = crate::native_artifact::compiler::frame_load(
                b,
                frame,
                core::mem::offset_of!(
                    crate::native_artifact::program::protocol::abi::Frame,
                    helper
                ),
            );
            let result = crate::native_artifact::compiler::frame_load(
                b,
                frame,
                core::mem::offset_of!(
                    crate::native_artifact::program::protocol::abi::Frame,
                    result
                ),
            );
            let arguments = crate::native_artifact::compiler::frame_load(
                b,
                frame,
                core::mem::offset_of!(
                    crate::native_artifact::program::protocol::abi::Frame,
                    arguments
                ),
            );
            let argument_capacity = crate::native_artifact::compiler::frame_load(
                b,
                frame,
                core::mem::offset_of!(
                    crate::native_artifact::program::protocol::abi::Frame,
                    argument_capacity
                ),
            );
            for pointer in [function, result, arguments] {
                let valid = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
                guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
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
        let operation = GlobalHelper::decode(operation);
        #[cfg(feature = "canonical-native-program")]
        {
            let count = b.ins().iconst(types::I64, operands.len() as i64);
            let valid = b.ins().icmp(
                IntCC::UnsignedGreaterThanOrEqual,
                self.argument_capacity,
                count,
            );
            guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
            for (index, &operand) in operands.iter().enumerate() {
                let value = if is_scalar(operand) {
                    crate::native_artifact::control_compiler::load(b, locals, operand)?
                } else {
                    // Managed operands are fetched from their exact generation
                    // in the canonical Value arena, never from native pointers.
                    b.ins().iconst(types::I64, 0)
                };
                b.ins().store(
                    cranelift_codegen::ir::MemFlags::new(),
                    value,
                    self.arguments,
                    crate::native_artifact::compiler::offset(index)?,
                );
            }
            let zero = b.ins().iconst(types::I64, 0);
            self.call(b, site, operation, [self.arguments, count, zero], error);
            Ok(())
        }
        #[cfg(not(feature = "canonical-native-program"))]
        {
            let _ = (b, site, operation, operands, locals, is_scalar, error);
            Err(crate::native_artifact::compiler::backend(
                "library operand ABI is not enabled",
            ))
        }
    }
    pub fn call(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
        operation: GlobalHelper,
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
        operation: GlobalHelper,
        args: [Value; 3],
        error: Block,
    ) {
        let operation = b.ins().iconst(types::I64, operation.raw() as i64);
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
        let valid = b.ins().icmp_imm(
            IntCC::Equal,
            status,
            crate::native_artifact::abi::STATUS_OK as i64,
        );
        guard(
            b,
            valid,
            error,
            crate::native_artifact::abi::STATUS_ADMISSION,
        );
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
                GlobalHelper::Collection,
                [args[0], args[1], zero],
                error,
            );
            Ok(())
        }
        #[cfg(not(feature = "canonical-native-program"))]
        {
            let _ = (b, site, args, error);
            Err(crate::native_artifact::compiler::backend(
                "collection ABI is not enabled",
            ))
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
        let operation = GlobalHelper::output(ordinal as u64).ok_or_else(|| {
            crate::native_artifact::compiler::backend("output helper ordinal overflow")
        })?;
        let zero = b.ins().iconst(types::I64, 0);
        self.call(b, site, operation, [value, zero, zero], error);
        Ok(())
    }
}
