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
