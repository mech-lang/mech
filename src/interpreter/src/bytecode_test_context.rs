use mech_core::{BytecodeCompilerContext, EncodedInstr, FeatureFlag, MResult, Register, ValueKind};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct RecordingBytecodeCompilerContext {
    pub reg_map: HashMap<usize, Register>,
    pub initialized_ptrs: HashSet<usize>,
    pub requirements: HashSet<FeatureFlag>,
    pub instructions: Vec<EncodedInstr>,
    pub const_count: u32,
    next_register: Register,
}

impl BytecodeCompilerContext for RecordingBytecodeCompilerContext {
    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool) {
        let register = match self.reg_map.get(&pointer) {
            Some(register) => *register,
            None => {
                let register = self.next_register;
                self.next_register += 1;
                self.reg_map.insert(pointer, register);
                register
            }
        };
        let needs_initialization = self.initialized_ptrs.insert(pointer);
        (register, needs_initialization)
    }

    fn compile_const(&mut self, _bytes: &[u8], _kind: ValueKind) -> MResult<u32> {
        let constant = self.const_count;
        self.const_count += 1;
        Ok(constant)
    }

    fn define_symbol(&mut self, _pointer: usize, _register: Register, _name: &str, _mutable: bool) {
    }

    fn require(&mut self, requirement: FeatureFlag) {
        self.requirements.insert(requirement);
    }

    fn emit_const_load(&mut self, destination: Register, constant: u32) {
        self.instructions.push(EncodedInstr::ConstLoad {
            dst: destination,
            const_id: constant,
        });
    }

    fn emit_nullop(&mut self, function: u64, destination: Register) {
        self.instructions.push(EncodedInstr::NullOp {
            fxn_id: function,
            dst: destination,
        });
    }

    fn emit_unop(&mut self, function: u64, destination: Register, source: Register) {
        self.instructions.push(EncodedInstr::UnOp {
            fxn_id: function,
            dst: destination,
            src: source,
        });
    }

    fn emit_binop(&mut self, function: u64, destination: Register, lhs: Register, rhs: Register) {
        self.instructions.push(EncodedInstr::BinOp {
            fxn_id: function,
            dst: destination,
            lhs,
            rhs,
        });
    }

    fn emit_ternop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
    ) {
        self.instructions.push(EncodedInstr::TernOp {
            fxn_id: function,
            dst: destination,
            a,
            b,
            c,
        });
    }

    fn emit_quadop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
        d: Register,
    ) {
        self.instructions.push(EncodedInstr::QuadOp {
            fxn_id: function,
            dst: destination,
            a,
            b,
            c,
            d,
        });
    }

    fn emit_varop(&mut self, function: u64, destination: Register, arguments: Vec<Register>) {
        self.instructions.push(EncodedInstr::VarArg {
            fxn_id: function,
            dst: destination,
            args: arguments,
        });
    }

    fn emit_ret(&mut self, source: Register) {
        self.instructions.push(EncodedInstr::Ret { src: source });
    }
}
