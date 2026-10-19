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
            for pointer in [function, result] {
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
            }))
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
