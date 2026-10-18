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
            signature.params = vec![cranelift_codegen::ir::AbiParam::new(types::I64); 6];
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
        args: [Value; 2],
        error: Block,
    ) {
        let site = b.ins().iconst(types::I64, site as i64);
        let operation = b.ins().iconst(types::I64, operation as i64);
        let call = b.ins().call_indirect(
            self.signature,
            self.function,
            &[self.context, site, operation, args[0], args[1], self.result],
        );
        let status = b.inst_results(call)[0];
        let valid = b
            .ins()
            .icmp_imm(IntCC::Equal, status, super::abi::STATUS_OK as i64);
        guard(b, valid, error, super::abi::STATUS_ADMISSION);
    }
}
