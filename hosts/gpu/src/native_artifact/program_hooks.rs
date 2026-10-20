//! Emission of explicit leaf/transport hooks, never a graph evaluator.
use super::NativeCompileError;
use super::compiler::guard;
use cranelift_codegen::ir::{Block, InstBuilder, SigRef, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;

pub(super) struct Hooks {
    signature: SigRef,
    function: Value,
    context: Value,
    pub result: Value,
    #[cfg(feature = "canonical-native-program")]
    arguments: Value,
    #[cfg(feature = "canonical-native-program")]
    argument_capacity: Value,
}
impl Hooks {
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
            self.call(
                b,
                site,
                super::program_abi::LIBRARY,
                [self.arguments, count, zero],
                error,
            );
            Ok(())
        }
        #[cfg(not(feature = "canonical-native-program"))]
        {
            let _ = (b, site, operands, locals, is_scalar, error);
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
